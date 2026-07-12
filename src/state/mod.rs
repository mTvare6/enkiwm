pub mod backend;
pub mod enki;

use smithay::{
    backend::input::{AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent},
    input::{
        keyboard::{FilterResult, Keysym},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    reexports::wayland_server::{
        backend::{ClientData, ClientId, DisconnectReason},
        protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::math::IVec2;

pub struct State {
    pub enki: enki::Enki,
    pub backend: backend::Backend,
}

impl State {
    pub fn new(event_loop: &mut smithay::reexports::calloop::EventLoop<'static, Self>, display: smithay::reexports::wayland_server::Display<Self>) -> Self {
        let mut backend = if std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some() {
            backend::Backend::Winit(backend::winit::WinitData::new(event_loop).unwrap())
        } else {
            backend::Backend::Udev(backend::udev::UdevData::new(event_loop).unwrap())
        };
        let mut enki = enki::Enki::new(display, event_loop, &backend.seat_name());
        //let mut backend =
        //    backend::Backend::Winit(backend::winit::WinitData::new(event_loop).unwrap());

        backend.init(event_loop, &mut enki).unwrap();

        Self {
            enki,
            backend,
        }
    }

    pub fn set_cursor_focus(&mut self) {
        let keyboard = self.enki.seat.get_keyboard().unwrap();
        let serial = SERIAL_COUNTER.next_serial();

        if let Some(target_window) = self.enki.grid.get(&self.enki.cell_cursor) {
            keyboard.set_focus(self, Some(target_window.toplevel().unwrap().wl_surface().clone()), serial);

            self.enki.space.elements().for_each(|w| {
                let in_marked_cell = w == &target_window;
                if w.set_activated(in_marked_cell) {
                    w.toplevel().unwrap().send_pending_configure();
                }
            });
        } else {
            keyboard.set_focus(self, None, serial);
            self.enki.space.elements().for_each(|w| {
                if w.set_activated(false) {
                    w.toplevel().unwrap().send_pending_configure();
                }
            });
        }
    }

    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.enki.surface_under(pos)
    }

    pub fn base_monitor_size(&self) -> IVec2 {
        self.enki.base_monitor_size()
    }

    pub fn update_viewport(&mut self, modal_change: bool) {
        self.enki.update_viewport(modal_change);
    }

    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);

                self.enki
                    .seat
                    .get_keyboard()
                    .unwrap()
                    .input::<(), _>(self, event.key_code(), event.state(), serial, time, |data, modifiers, handle| {
                        if event.state() == KeyState::Pressed {
                            let sym = handle.modified_sym();
                            if sym == Keysym::Alt_R {
                                data.enki.modal_mode = !data.enki.modal_mode;
                                data.enki.update_viewport(true);
                                return FilterResult::Intercept(());
                            }
                            if data.enki.modal_mode {
                                let dir_flipped = match sym {
                                    Keysym::l | Keysym::L => Some(IVec2::X),
                                    Keysym::h | Keysym::H => Some(IVec2::NEG_X),
                                    Keysym::j | Keysym::J => Some(IVec2::NEG_Y),
                                    Keysym::k | Keysym::K => Some(IVec2::Y),
                                    _ => None,
                                };
                                if let Some(dir_flipped) = dir_flipped {
                                    let dir = dir_flipped * IVec2::FLIP_Y;
                                    let grow_dir = |span: &mut i32, origin: &mut i32, d: i32| {
                                        *span += d.abs();
                                        *origin -= (d < 0) as i32;
                                    };
                                    let shrink_dir = |span: &mut i32, origin: &mut i32, d: i32| {
                                        let old = *span;
                                        *span = (*span - d.abs()).max(1);
                                        if *span < old {
                                            *origin += (d < 0) as i32;
                                        }
                                    };
                                    let camera = &mut data.enki.camera;
                                    if modifiers.shift {
                                        grow_dir(&mut camera.span.x, &mut camera.origin.x, dir.x);
                                        grow_dir(&mut camera.span.y, &mut camera.origin.y, dir.y);
                                    } else if modifiers.alt {
                                        shrink_dir(&mut camera.span.x, &mut camera.origin.x, dir.x);
                                        shrink_dir(&mut camera.span.y, &mut camera.origin.y, dir.y);
                                    } else {
                                        camera.origin += dir;
                                    }
                                    data.enki.update_viewport(false);
                                    return FilterResult::Intercept(());
                                }
                                let focus_dir_flipped = match sym {
                                    Keysym::u | Keysym::U => Some(IVec2::NEG_X),
                                    Keysym::i | Keysym::I => Some(IVec2::NEG_Y),
                                    Keysym::o | Keysym::O => Some(IVec2::Y),
                                    Keysym::p | Keysym::P => Some(IVec2::X),
                                    _ => None,
                                };
                                if let Some(focus_dir) = focus_dir_flipped {
                                    let dir = focus_dir * IVec2::FLIP_Y;
                                    if modifiers.shift {
                                        let target_loc = data.enki.cell_cursor + dir;
                                        data.enki.grid.swap(data.enki.cell_cursor, target_loc);
                                        data.enki.cell_cursor = target_loc;
                                        data.enki.update_viewport(false);
                                    } else {
                                        data.enki.cell_cursor += dir;
                                    }
                                    data.set_cursor_focus();
                                    return FilterResult::Intercept(());
                                }
                                let program = match sym {
                                    Keysym::Return if modifiers.shift => Some("weston-terminal"),
                                    Keysym::Return => Some("kitty"),
                                    Keysym::w => Some("firefox"),
                                    _ => None,
                                };
                                if let Some(program) = program {
                                    std::process::Command::new(program).spawn().ok();
                                }
                                return FilterResult::Intercept(());
                            }
                        }
                        FilterResult::Forward
                    });
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.enki.space.outputs().next().unwrap();
                let output_geo = self.enki.space.output_geometry(output).unwrap();
                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.enki.seat.get_pointer().unwrap();
                let under = self.surface_under(pos);
                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.enki.seat.get_pointer().unwrap();
                let keyboard = self.enki.seat.get_keyboard().unwrap();
                let serial = SERIAL_COUNTER.next_serial();
                let button = event.button_code();
                let button_state = event.state();

                if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
                    if let Some((window, _loc)) = self.enki.space.element_under(pointer.current_location()).map(|(w, l)| (w.clone(), l)) {
                        self.enki.space.raise_element(&window, true);
                        keyboard.set_focus(self, Some(window.toplevel().unwrap().wl_surface().clone()), serial);
                        self.enki.space.elements().for_each(|window| {
                            window.toplevel().unwrap().send_pending_configure();
                        });
                    } else {
                        self.enki.space.elements().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().unwrap().send_pending_configure();
                        });
                        keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                    }
                }

                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event, .. } => {
                let source = event.source();
                let horizontal_amount = event.amount(Axis::Horizontal).unwrap_or_else(|| event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.);
                let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.);
                let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
                let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.v120(Axis::Horizontal, discrete as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.v120(Axis::Vertical, discrete as i32);
                    }
                }

                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                let pointer = self.enki.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: smithay::wayland::compositor::CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
