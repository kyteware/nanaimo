use smithay::{
    wayland::{
        compositor::{CompositorState, CompositorHandler, CompositorClientState},
        buffer::BufferHandler,
    },
    reexports::wayland_server::protocol::{wl_surface::WlSurface, wl_buffer::WlBuffer},
};

use crate::state::{NanaimoState, ClientState};

impl BufferHandler for NanaimoState {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for NanaimoState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
    
    fn client_compositor_state<'a>(&self, client: &'a smithay::reexports::wayland_server::Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        tracing::trace!("Surface commit: {:?}", surface);
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
        if let Some(window) = self.space.elements().find(|w| w.toplevel().map(|tl| tl.wl_surface() == surface).unwrap_or(false)) {
             tracing::trace!("Window commit: {:?}", window);
             window.on_commit();
        }
    }
}
