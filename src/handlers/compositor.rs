use crate::{
    grabs::resize_grab,
    state::{ClientState, State},
};
use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_shm,
    reexports::wayland_server::{
        protocol::{wl_buffer, wl_surface::WlSurface},
        Client,
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler, CompositorState},
        shm::{ShmHandler, ShmState},
    },
};

use super::xdg_shell;

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState { &mut self.enki.compositor_state }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState { &client.get_data::<ClientState>().unwrap().compositor_state }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.enki.space.elements().find(|w| w.toplevel().unwrap().wl_surface() == &root) {
                window.on_commit();
            }
        };

        xdg_shell::handle_commit(&mut self.enki.popups, &self.enki.space, surface);
        resize_grab::handle_commit(&mut self.enki.space, surface);
    }
}

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState { &self.enki.shm_state }
}

delegate_compositor!(State);
delegate_shm!(State);
