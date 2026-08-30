use std::{ffi::OsString, sync::Arc};

use smithay::{
    desktop::{find_popup_root_surface, get_popup_toplevel_coords, PopupKind, PopupManager, Space, Window, WindowSurfaceType},
    input::{pointer::CursorImageStatus, Seat, SeatState},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopHandle, LoopSignal, Mode, PostAction},
        wayland_server::{protocol::wl_surface::WlSurface, Display, DisplayHandle},
    },
    utils::{Logical, Point},
    wayland::{
        compositor::CompositorState,
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::{PopupSurface, XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use crate::{cursor::Cursor, layout::Grid, math::IVec2, state::State};

pub struct Camera {
    pub origin: IVec2,
    pub span: IVec2,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            origin: IVec2::ZERO,
            span: IVec2::ONE,
        }
    }
}

pub struct Enki {
    pub start_time: std::time::Instant,
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub loop_handle: LoopHandle<'static, State>,

    // smithay state
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<State>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
    pub seat: Seat<State>,

    // cursor
    pub cursor_image_status: CursorImageStatus,

    // window layout
    pub space: Space<Window>,
    pub grid: Grid,
    pub modal_mode: bool,
    pub camera: Camera,
    pub cell_cursor: IVec2,
}

impl Enki {
    pub fn new(display: Display<super::State>, event_loop: &mut EventLoop<'static, State>, seat_name: &str) -> Self {
        let start_time = std::time::Instant::now();
        let dh = display.handle();

        let compositor_state = CompositorState::new::<super::State>(&dh);
        let xdg_shell_state = XdgShellState::new::<super::State>(&dh);
        let shm_state = ShmState::new::<super::State>(&dh, vec![]);
        let popups = PopupManager::default();
        let output_manager_state = OutputManagerState::new_with_xdg_output::<super::State>(&dh);
        let data_device_state = DataDeviceState::new::<super::State>(&dh);

        let mut seat_state = SeatState::new();
        // seat name needs to be changed for udev!!
        let mut seat: Seat<super::State> = seat_state.new_wl_seat(&dh, seat_name);
        seat.add_keyboard(Default::default(), 200, 50).unwrap();
        seat.add_pointer();

        let socket_name = Self::init_wayland_listener(display, event_loop);
        let loop_signal = event_loop.get_signal();

        Self {
            loop_handle: event_loop.handle(),
            start_time,
            socket_name,
            display_handle: dh,
            loop_signal,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            popups,
            cursor_image_status: CursorImageStatus::default_named(),
            seat,
            space: Space::default(),
            grid: Grid::new(),
            modal_mode: false,
            camera: Camera::new(),
            cell_cursor: IVec2::ZERO,
        }
    }

    fn init_wayland_listener(display: Display<super::State>, event_loop: &mut EventLoop<super::State>) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();
        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                state.enki.display_handle.insert_client(client_stream, Arc::new(super::ClientState::default())).unwrap();
            })
            .expect("Failed to init the wayland event source.");

        loop_handle
            .insert_source(Generic::new(display, Interest::READ, Mode::Level), |_, display, state| {
                unsafe {
                    display.get_mut().dispatch_clients(state).unwrap();
                }
                Ok(PostAction::Continue)
            })
            .unwrap();

        socket_name
    }

    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| window.surface_under(pos - location.to_f64(), WindowSurfaceType::ALL).map(|(s, p)| (s, (p + location).to_f64())))
    }

    pub fn base_monitor_size(&self) -> IVec2 {
        let output = self.space.outputs().next().unwrap();
        let mode = output.current_mode().unwrap();
        mode.size.into()
    }

    pub fn update_viewport(&mut self, modal_change: bool) {
        let output = self.space.outputs().next().unwrap().clone();
        let monitor_size = self.base_monitor_size();
        let cell_size = monitor_size / self.camera.span;

        for (&grid_idx, window) in self.grid.cells.iter() {
            let pixel_loc = grid_idx * cell_size;
            self.space.map_element(window.clone(), pixel_loc, false);

            window.toplevel().unwrap().with_pending_state(|state| {
                state.size = Some(cell_size.into());
            });
            window.toplevel().unwrap().send_pending_configure();
        }

        let scale = if self.modal_mode { 0.75 } else { 1.0 };

        if modal_change {
            output.change_current_state(None, None, Some(smithay::output::Scale::Fractional(scale)), None);
        }

        let logical_size = IVec2::new((monitor_size.x as f64 / scale) as i32, (monitor_size.y as f64 / scale) as i32);

        let offset = (logical_size - monitor_size) / 2;
        let base_loc = self.camera.origin * cell_size;

        self.space.map_output(&output, base_loc - offset);
    }

    pub fn unconstrain_popup(&self, popup: &PopupSurface) {
        let Ok(root) = find_popup_root_surface(&PopupKind::Xdg(popup.clone())) else {
            return;
        };
        let Some(window) = self.space.elements().find(|w| w.toplevel().unwrap().wl_surface() == &root) else {
            return;
        };

        let output = self.space.outputs().next().unwrap();
        let output_geo = self.space.output_geometry(output).unwrap();
        let window_geo = self.space.element_geometry(window).unwrap();

        let mut target = output_geo;
        target.loc -= get_popup_toplevel_coords(&PopupKind::Xdg(popup.clone()));
        target.loc -= window_geo.loc;

        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }
}
