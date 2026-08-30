use std::time::Duration;

use smithay::{
    backend::{
        renderer::{damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement, gles::GlesRenderer},
        winit::{self, WinitEvent, WinitGraphicsBackend},
    },
    desktop::{Space, Window},
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::EventLoop,
        winit::{dpi::LogicalSize, window::WindowAttributes},
    },
    utils::Rectangle,
};

use crate::state::{backend::RenderResult, enki::Enki, State};

pub struct WinitData {
    pub(super) output: Output,
    pub(super) backend: WinitGraphicsBackend<GlesRenderer>,
    pub(super) damage: OutputDamageTracker,
}

impl WinitData {
    pub fn new(event_loop: &EventLoop<State>) -> Result<Self, Box<dyn std::error::Error>> {
        let (backend, winit) = winit::init_from_attributes(WindowAttributes::default().with_inner_size(LogicalSize::new(1280, 720)))?;

        let output = Output::new(
            "winit".to_string(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Smithay".into(),
                model: "Winit".into(),
                serial_number: "Unknown".into(),
            },
        );

        let mode = Mode {
            size: backend.window_size(),
            refresh: 60_000,
        };

        output.change_current_state(Some(mode), Some(smithay::utils::Transform::Flipped180), None, Some((0, 0).into()));
        output.set_preferred(mode);

        let damage_tracker = OutputDamageTracker::from_output(&output);

        let output_for_closure = output.clone();

        event_loop
            .handle()
            .insert_source(winit, move |event, _, state| match event {
                WinitEvent::Resized { size, .. } => {
                    output_for_closure.change_current_state(
                        Some(Mode {
                            size,
                            refresh: 60_000,
                        }),
                        None,
                        None,
                        None,
                    );
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Redraw => {
                    state.backend.render_frame(&state.enki.space);

                    state.enki.space.elements().for_each(|window| window.send_frame(&output_for_closure, state.enki.start_time.elapsed(), Some(Duration::ZERO), |_, _| Some(output_for_closure.clone())));

                    state.enki.space.refresh();
                    state.enki.popups.cleanup();
                    state.enki.grid.cleanup();
                    let _ = state.enki.display_handle.flush_clients();
                }
                WinitEvent::CloseRequested => {
                    state.enki.loop_signal.stop();
                }
                _ => (),
            })
            .unwrap();

        Ok(WinitData {
            output,
            backend,
            damage: damage_tracker,
        })
    }

    pub fn init(&self, enki: &mut Enki) -> Result<(), Box<dyn std::error::Error>> {
        let dh = enki.display_handle.clone();
        self.output.create_global::<State>(&dh);
        enki.space.map_output(&self.output, (0, 0));

        Ok(())
    }

    pub fn render(&mut self, space: &Space<Window>) -> RenderResult {
        let size = self.backend.window_size();
        let damage = Rectangle::from_size(size);

        {
            let (renderer, mut framebuffer) = self.backend.bind().unwrap();
            smithay::desktop::space::render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(&self.output, renderer, &mut framebuffer, 1.0, 0, [space], &[], &mut self.damage, [0.1, 0.1, 0.1, 1.0]).unwrap();
        }

        self.backend.submit(Some(&[damage])).unwrap();
        self.backend.window().request_redraw();
        RenderResult::Submitted
    }
}
