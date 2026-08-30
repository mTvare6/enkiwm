use smithay::{
    delegate_xdg_shell,
    desktop::{PopupKind, PopupManager, Space, Window},
    input::{
        pointer::{Focus, GrabStartData as PointerGrabStartData},
        Seat,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            protocol::{wl_seat, wl_surface::WlSurface},
            Resource,
        },
    },
    utils::{Rectangle, Serial},
    wayland::{
        compositor::with_states,
        shell::xdg::{PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState, XdgToplevelSurfaceData},
    },
};

use crate::{
    grabs::{MoveSurfaceGrab, ResizeSurfaceGrab},
    state::State,
};

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.enki.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface.clone());

        let monitor_size = self.enki.base_monitor_size();

        let cell_size = monitor_size / self.enki.camera.span;

        let start_index = self.enki.camera.origin;
        let target_index = self.enki.grid.find_nearest_empty(start_index);

        let pixel_position = target_index * cell_size;

        self.enki.space.map_element(window.clone(), pixel_position, false);
        self.enki.grid.insert(target_index, window);

        surface.with_pending_state(|state| {
            state.size = Some(cell_size.into());
            state.states.set(xdg_toplevel::State::TiledTop);
            state.states.set(xdg_toplevel::State::TiledBottom);
            state.states.set(xdg_toplevel::State::TiledLeft);
            state.states.set(xdg_toplevel::State::TiledRight);
        });

        surface.send_configure();
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        self.enki.unconstrain_popup(&surface);
        let _ = self.enki.popups.track_popup(PopupKind::Xdg(surface));
    }

    fn reposition_request(&mut self, surface: PopupSurface, positioner: PositionerState, token: u32) {
        surface.with_pending_state(|state| {
            let geometry = positioner.get_geometry();
            state.geometry = geometry;
            state.positioner = positioner;
        });
        self.enki.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial) {
        let seat = Seat::<State>::from_resource(&seat).unwrap();

        let wl_surface = surface.wl_surface();

        if let Some(start_data) = check_grab(&seat, wl_surface, serial) {
            let pointer = seat.get_pointer().unwrap();

            let window = self.enki.space.elements().find(|w| w.toplevel().unwrap().wl_surface() == wl_surface).unwrap().clone();
            let initial_window_location = self.enki.space.element_location(&window).unwrap();

            let grab = MoveSurfaceGrab {
                start_data,
                window,
                initial_window_location,
            };

            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }

    fn resize_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial, edges: xdg_toplevel::ResizeEdge) {
        let seat = Seat::from_resource(&seat).unwrap();

        let wl_surface = surface.wl_surface();

        if let Some(start_data) = check_grab(&seat, wl_surface, serial) {
            let pointer = seat.get_pointer().unwrap();

            let window = self.enki.space.elements().find(|w| w.toplevel().unwrap().wl_surface() == wl_surface).unwrap().clone();
            let initial_window_location = self.enki.space.element_location(&window).unwrap();
            let initial_window_size = window.geometry().size;

            surface.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Resizing);
            });

            surface.send_pending_configure();

            let grab = ResizeSurfaceGrab::start(start_data, window, edges.into(), Rectangle::new(initial_window_location, initial_window_size));

            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}
}

delegate_xdg_shell!(State);

fn check_grab(seat: &Seat<State>, surface: &WlSurface, serial: Serial) -> Option<PointerGrabStartData<State>> {
    let pointer = seat.get_pointer()?;

    if !pointer.has_grab(serial) {
        return None;
    }

    let start_data = pointer.grab_start_data()?;

    let (focus, _) = start_data.focus.as_ref()?;
    if !focus.id().same_client_as(&surface.id()) {
        return None;
    }

    Some(start_data)
}

pub fn handle_commit(popups: &mut PopupManager, space: &Space<Window>, surface: &WlSurface) {
    if let Some(window) = space.elements().find(|w| w.toplevel().unwrap().wl_surface() == surface).cloned() {
        let initial_configure_sent = with_states(surface, |states| states.data_map.get::<XdgToplevelSurfaceData>().unwrap().lock().unwrap().initial_configure_sent);

        if !initial_configure_sent {
            window.toplevel().unwrap().send_configure();
        }
    }

    popups.commit(surface);
    if let Some(popup) = popups.find_popup(surface) {
        match popup {
            PopupKind::Xdg(ref xdg) => {
                if !xdg.is_initial_configure_sent() {
                    xdg.send_configure().expect("initial configure failed");
                }
            }
            PopupKind::InputMethod(ref _input_method) => {}
        }
    }
}
