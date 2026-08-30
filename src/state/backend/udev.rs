use smithay::{
    backend::{
        allocator::{
            format::FormatSet,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            compositor::FrameFlags,
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements},
            DrmDevice, DrmDeviceFd, DrmEvent, DrmEventMetadata, DrmNode,
        },
        egl::{EGLDevice, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            element::{memory::MemoryRenderBuffer, surface::WaylandSurfaceRenderElement, AsRenderElements},
            gles::GlesRenderer,
            multigpu::{gbm::GbmGlesBackend, GpuManager, MultiRenderer},
            ImportAll, ImportMem,
        },
        session::{libseat::LibSeatSession, Event as SessionEvent, Session},
        udev::{self, UdevBackend, UdevEvent},
    },
    desktop::{space::SpaceRenderElements, utils::OutputPresentationFeedback, Window},
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::{Mode as WlMode, Output, PhysicalProperties},
    reexports::{
        calloop::{EventLoop, RegistrationToken},
        drm::control::{connector, crtc, ModeTypeFlags},
        gbm::Modifier,
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::backend::GlobalId,
    },
    render_elements,
    utils::{DeviceFd, IsAlive, Scale},
    wayland::compositor,
};

use std::{collections::HashMap, fmt::Write, path::Path, sync::Mutex};

use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};

use tracing::{error, info, warn, warn_span};

use crate::{
    cursor::*,
    state::{enki::Enki, State},
};

type UdevRenderer<'a> = MultiRenderer<'a, 'a, GbmGlesBackend<GlesRenderer, DrmDeviceFd>, GbmGlesBackend<GlesRenderer, DrmDeviceFd>>;

type UdevOutputManager = DrmOutputManager<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, Option<OutputPresentationFeedback>, DrmDeviceFd>;

type UdevDrmOutput = DrmOutput<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, Option<OutputPresentationFeedback>, DrmDeviceFd>;

render_elements! {
    OutputRenderElements<R, E> where R: ImportAll + ImportMem;
    Pointer = PointerRenderElement<R>,
    Window = SpaceRenderElements<R, E>
}

const SUPPORTED_FORMATS: &[Fourcc] = &[
    Fourcc::Abgr2101010,
    Fourcc::Argb2101010,
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];
const SUPPORTED_FORMATS_8BIT_ONLY: &[Fourcc] = &[
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];

// Represents a gpu
pub struct DeviceData {
    output_manager: UdevOutputManager,
    drm_scanner: DrmScanner,
    render_node: Option<DrmNode>,
    surfaces: HashMap<crtc::Handle, OutputData>,
    registration_token: RegistrationToken,
}

// Represents a monitor
pub struct OutputData {
    output: Output,
    drm_output: UdevDrmOutput,
    global: GlobalId,
    is_pending: bool,
}

pub struct UdevData {
    pub(super) session: LibSeatSession,
    input: Libinput,

    // Devices
    gpu_manager: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    primary_gpu: DrmNode,
    devices: HashMap<DrmNode, DeviceData>,

    // Cursor stuff
    cursor: Cursor,
    cursor_images: Vec<(xcursor::parser::Image, MemoryRenderBuffer)>,
    pointer_element: PointerElement,
}

impl UdevData {
    pub fn new(event_loop: &EventLoop<State>) -> Result<Self, Box<dyn std::error::Error>> {
        let (session, notifier) = LibSeatSession::new()?;

        let mut lib_input = Libinput::new_with_udev(LibinputSessionInterface::from(session.clone()));
        lib_input.udev_assign_seat(&session.seat()).unwrap();

        let input_backend = LibinputInputBackend::new(lib_input.clone());

        // Insert the input sources
        event_loop.handle().insert_source(input_backend, |event, _, state| {
            state.process_input_event(event);
        })?;

        event_loop
            .handle()
            .insert_source(notifier, move |event, _, state| match event {
                // TTY switching needs to be handled here.
                SessionEvent::ActivateSession => {
                    info!("Session activated, becomong the DRM master ");

                    state.backend.udev().input.resume().unwrap();

                    for device in state.backend.udev().devices.values_mut() {
                        if let Err(err) = device.output_manager.device_mut().activate(true) {
                            tracing::error!("Failed to acquire DRM master on activate: field {:?}", err);
                        }
                    }

                    state.enki.space.refresh();
                }
                SessionEvent::PauseSession => {
                    state.backend.udev().input.suspend();
                }
            })
            .unwrap();

        let api = GbmGlesBackend::with_context_priority(smithay::backend::egl::context::ContextPriority::High);

        let gpu_manager = GpuManager::new(api)?;

        let (primary_gpu, primary_render_node) = {
            let primary_gpu_path = udev::primary_gpu(&session.seat())?.unwrap();
            let primary_node = DrmNode::from_path(&primary_gpu_path)?;
            let primary_render_node = primary_node.node_with_type(smithay::backend::drm::NodeType::Render).and_then(Result::ok).unwrap_or_else(|| {
                warn_span!("Error getting the Render Node for the primary GPU; Proceeding anyway");
                primary_node
            });

            (primary_node, primary_render_node)
        };

        let mut node_path = String::new();
        if let Some(path) = primary_render_node.dev_path() {
            write!(node_path, "{path:?}").unwrap();
        } else {
            write!(node_path, "{primary_render_node}").unwrap();
        }

        info!("Using as the render node: {node_path}.");

        Ok(Self {
            session,
            input: lib_input,
            gpu_manager,
            primary_gpu,
            devices: HashMap::new(),
            cursor: Cursor::load(),
            cursor_images: vec![],
            pointer_element: PointerElement::default(),
        })
    }

    pub fn init(&mut self, event_loop: &EventLoop<State>, enki: &mut Enki) -> Result<(), Box<dyn std::error::Error>> {
        self.pointer_element.set_status(enki.cursor_image_status.clone());

        let udev_backend = UdevBackend::new(self.session.seat())?;

        for (device_id, path) in udev_backend.device_list() {
            let Ok(node) = DrmNode::from_dev_id(device_id) else {
                continue;
            };
            if let Err(err) = self.device_added(node, path, enki) {
                warn!("Failed to add device {node}: {err}");
                continue;
            }
            self.device_changed(node, enki);
        }

        event_loop.handle().insert_source(udev_backend, move |event, _, state| match event {
            UdevEvent::Added {
                device_id,
                path,
            } => {
                let Ok(node) = DrmNode::from_dev_id(device_id) else {
                    return;
                };
                let backend = state.backend.udev();
                if let Err(err) = backend.device_added(node, &path, &mut state.enki) {
                    warn!("Failed to add device {node}: {err}");
                    return;
                }
                backend.device_changed(node, &mut state.enki);
            }
            UdevEvent::Changed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    state.backend.udev().device_changed(node, &mut state.enki);
                }
            }
            UdevEvent::Removed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    state.backend.udev().device_removed(node, &mut state.enki);
                }
            }
        })?;

        Ok(())
    }
}

// These are all udev event functions
impl UdevData {
    fn device_added(&mut self, node: DrmNode, path: &Path, enki: &mut Enki) -> Result<(), Box<dyn std::error::Error>> {
        // Try to open the device
        let fd = self.session.open(path, OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK)?;

        let fd = DrmDeviceFd::new(DeviceFd::from(fd));

        let (drm, notifier) = DrmDevice::new(fd.clone(), true)?;
        let gbm = GbmDevice::new(fd)?;

        let registration_token = enki
            .loop_handle
            .insert_source(notifier, move |event, metadata, data: &mut State| match event {
                DrmEvent::VBlank(crtc) => {
                    data.backend.udev().on_vblank(node, crtc, metadata, &mut data.enki);
                }
                DrmEvent::Error(error) => {
                    error!("{:?}", error);
                }
            })
            .unwrap();

        let mut try_initialize_gpu = || -> Result<DrmNode, Box<dyn std::error::Error>> {
            let display = unsafe { EGLDisplay::new(gbm.clone())? };
            let egl_device = EGLDevice::device_for_display(&display)?;

            let render_node = egl_device.try_get_render_node().ok().flatten().unwrap_or(node);
            self.gpu_manager.as_mut().add_node(render_node, gbm.clone())?;
            Ok(render_node)
        };

        let render_node = try_initialize_gpu()
            .inspect_err(|err| {
                warn!(?err, "Failed to initialize gpu");
            })
            .ok();

        let allocator = render_node
            .is_some()
            .then(|| GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT))
            .or_else(|| {
                self.devices
                    .get(&self.primary_gpu)
                    .or_else(|| self.devices.values().find(|backend| backend.render_node == Some(self.primary_gpu)))
                    .map(|backend| backend.output_manager.allocator().clone())
            });

        let framebuffer_exporter = GbmFramebufferExporter::new(gbm.clone(), render_node.into());

        let color_formats = if std::env::var("ANVIL_DISABLE_10BIT").is_ok() {
            SUPPORTED_FORMATS_8BIT_ONLY
        } else {
            SUPPORTED_FORMATS
        };

        let mut renderer = self.gpu_manager.single_renderer(&render_node.unwrap_or(self.primary_gpu)).unwrap();

        let render_formats = renderer
            .as_mut()
            .egl_context()
            .dmabuf_render_formats()
            .iter()
            .filter(|format| render_node.is_some() || format.modifier == Modifier::Linear)
            .copied()
            .collect::<FormatSet>();

        let drm_output_manager = DrmOutputManager::new(drm, allocator.unwrap(), framebuffer_exporter, Some(gbm), color_formats.iter().copied(), render_formats);

        self.devices.insert(
            node,
            DeviceData {
                registration_token,
                output_manager: drm_output_manager,
                render_node,
                drm_scanner: DrmScanner::new(),
                surfaces: HashMap::new(),
            },
        );

        self.device_changed(node, enki);

        Ok(())
    }

    fn device_changed(&mut self, node: DrmNode, enki: &mut Enki) {
        let Some(device) = self.devices.get_mut(&node) else {
            return;
        };

        let scan_result = match device.drm_scanner.scan_connectors(device.output_manager.device()) {
            Ok(result) => result,
            Err(err) => {
                warn!("Failed to scan connectors: {err}");
                return;
            }
        };

        for event in scan_result {
            match event {
                DrmScanEvent::Connected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    self.connector_connected(node, connector, crtc, enki);
                    self.render_surface(node, crtc, &enki);
                }
                DrmScanEvent::Disconnected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    self.connector_disconnected(node, connector, crtc, enki);
                }
                _ => {}
            }
        }
    }

    fn device_removed(&mut self, node: DrmNode, enki: &mut Enki) {
        let Some(device) = self.devices.remove(&node) else {
            return;
        };

        for (_, surface) in device.surfaces {
            enki.space.unmap_output(&surface.output);
        }

        enki.loop_handle.remove(device.registration_token);

        if let Some(render_node) = device.render_node {
            self.gpu_manager.as_mut().remove_node(&render_node);
        }
    }

    fn connector_connected(&mut self, node: DrmNode, connector: connector::Info, crtc: crtc::Handle, enki: &mut Enki) {
        let Some(device) = self.devices.get_mut(&node) else {
            return;
        };

        let mode_id = connector.modes().iter().position(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED)).unwrap_or(0);
        let Some(drm_mode) = connector.modes().get(mode_id).copied() else {
            warn!("Connector {:?} has no modes", connector.handle());
            return;
        };
        let wl_mode = WlMode::from(drm_mode);

        let (phys_w, phys_h) = connector.size().unwrap_or((0, 0));
        let output = Output::new(
            format!("{}-{}", connector.interface().as_str(), connector.interface_id()),
            PhysicalProperties {
                size: (phys_w as i32, phys_h as i32).into(),
                subpixel: connector.subpixel().into(),
                make: "Unknown".into(),
                model: "Unknown".into(),
                serial_number: "Unknown".into(),
            },
        );
        let global = output.create_global::<State>(&enki.display_handle);

        let x = enki.space.outputs().fold(0, |acc, o| acc + enki.space.output_geometry(o).unwrap().size.w);

        output.set_preferred(wl_mode);
        output.change_current_state(Some(wl_mode), None, None, Some((x, 0).into()));
        enki.space.map_output(&output, (x, 0));

        let render_node = device.render_node.unwrap_or(self.primary_gpu);
        let mut renderer = match self.gpu_manager.single_renderer(&render_node) {
            Ok(renderer) => renderer,
            Err(err) => {
                warn!("Failed to get renderer: {err}");
                return;
            }
        };

        let drm_output = match device.output_manager.lock().initialize_output::<_, WaylandSurfaceRenderElement<UdevRenderer<'_>>>(
            crtc,
            drm_mode,
            &[connector.handle()],
            &output,
            None,
            &mut renderer,
            &DrmOutputRenderElements::default(),
        ) {
            Ok(drm_output) => drm_output,
            Err(err) => {
                warn!("Failed to initialize DRM output: {err}");
                return;
            }
        };

        device.surfaces.insert(
            crtc,
            OutputData {
                output,
                global,
                drm_output,
                is_pending: false,
            },
        );
    }

    fn connector_disconnected(&mut self, node: DrmNode, _connector: connector::Info, crtc: crtc::Handle, enki: &mut Enki) {
        let Some(device) = self.devices.get_mut(&node) else {
            return;
        };
        if let Some(surface) = device.surfaces.remove(&crtc) {
            enki.space.unmap_output(&surface.output);
            enki.display_handle.remove_global::<State>(surface.global);
        }
    }
}

impl UdevData {
    pub fn render_all(&mut self, enki: &Enki) {
        let nodes: Vec<_> = self.devices.keys().copied().collect();
        for node in nodes {
            if let Some(device) = self.devices.get(&node) {
                let crtcs: Vec<_> = device.surfaces.keys().copied().collect();
                for crtc in crtcs {
                    let is_pending = {
                        let device = self.devices.get(&node).unwrap();
                        let surface = device.surfaces.get(&crtc).unwrap();
                        surface.is_pending
                    };
                    if !is_pending {
                        self.render_surface(node, crtc, enki);
                    }
                }
            }
        }
    }

    fn render_surface(&mut self, node: DrmNode, crtc: crtc::Handle, enki: &Enki) {
        let Some(device) = self.devices.get_mut(&node) else {
            return;
        };
        let Some(surface) = device.surfaces.get_mut(&crtc) else {
            return;
        };

        let render_node = device.render_node.unwrap_or(self.primary_gpu);
        let mut renderer = match self.gpu_manager.single_renderer(&render_node) {
            Ok(renderer) => renderer,
            Err(err) => {
                warn!("Failed to get renderer: {err}");
                return;
            }
        };

        let Some(output_geometry) = enki.space.output_geometry(&surface.output) else {
            return;
        };

        let scale = Scale::from(surface.output.current_scale().fractional_scale());

        // Cursor rendering

        let mut pointer_elements = vec![];

        let pointer_location = enki.seat.get_pointer().unwrap().current_location();
        if output_geometry.to_f64().contains(pointer_location) {
            let mut cursor_status = enki.cursor_image_status.clone();

            let hotspot = if let CursorImageStatus::Surface(ref surf) = cursor_status {
                if surf.alive() {
                    compositor::with_states(surf, |states| states.data_map.get::<Mutex<CursorImageAttributes>>().unwrap().lock().unwrap().hotspot)
                } else {
                    cursor_status = CursorImageStatus::default_named();
                    (0, 0).into()
                }
            } else {
                (0, 0).into()
            };

            let cursor_pos = pointer_location - output_geometry.loc.to_f64();

            if let CursorImageStatus::Named(_) = &cursor_status {
                let frame = self.cursor.get_image(1, enki.start_time.elapsed());
                let buffer = self.cursor_images.iter().find_map(|(img, buf)| (img == &frame).then(|| buf.clone())).unwrap_or_else(|| {
                    let buf = MemoryRenderBuffer::from_slice(&frame.pixels_rgba, Fourcc::Argb8888, (frame.width as i32, frame.height as i32), 1, smithay::utils::Transform::Normal, None);
                    self.cursor_images.push((frame, buf.clone()));
                    buf
                });
                self.pointer_element.set_buffer(buffer);
            }
            self.pointer_element.set_status(cursor_status);

            pointer_elements.extend(
                self.pointer_element
                    .render_elements::<PointerRenderElement<_>>(&mut renderer, (cursor_pos - hotspot.to_f64()).to_physical(scale).to_i32_round(), scale, 1.0),
            );
        }

        let space_elements = match smithay::desktop::space::space_render_elements::<_, Window, _>(&mut renderer, [&enki.space], &surface.output, 1.0) {
            Ok(elements) => elements,
            Err(err) => {
                warn!("Failed to collect render elements: {err:?}");
                return;
            }
        };

        let elements: Vec<_> = space_elements
            .into_iter()
            .map(|s| OutputRenderElements::Window(s))
            .chain(pointer_elements.into_iter().map(|p| OutputRenderElements::Pointer(p)))
            .collect();

        match surface.drm_output.render_frame(&mut renderer, &elements, [0.1, 0.1, 0.1, 1.0], FrameFlags::DEFAULT) {
            Ok(result) => {
                if !result.is_empty {
                    if let Err(err) = surface.drm_output.queue_frame(None) {
                        warn!("Failed to queue frame: {err:?}");
                    } else {
                        surface.is_pending = true;
                    }
                }
            }
            Err(err) => warn!("Render error: {err:?}"),
        }
    }

    fn on_vblank(&mut self, node: DrmNode, crtc: crtc::Handle, _metadata: &mut Option<DrmEventMetadata>, enki: &mut Enki) {
        if let Some(device) = self.devices.get_mut(&node) {
            if let Some(surface) = device.surfaces.get_mut(&crtc) {
                surface.is_pending = false;
                if let Err(err) = surface.drm_output.frame_submitted() {
                    warn!("Frame_submitted error: {err:?}");
                }

                let output = surface.output.clone();
                let time = enki.start_time.elapsed();
                enki.space.elements().for_each(|window| {
                    window.send_frame(&output, time, Some(std::time::Duration::ZERO), |_, _| Some(output.clone()));
                });
                enki.space.refresh();
                enki.popups.cleanup();
                enki.grid.cleanup();
                let _ = enki.display_handle.flush_clients();
            }
        }

        self.render_surface(node, crtc, &enki);
    }
}
