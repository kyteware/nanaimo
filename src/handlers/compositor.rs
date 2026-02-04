use smithay::{
    reexports::wayland_server::protocol::{wl_buffer::WlBuffer, wl_surface::WlSurface},
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
    },
};
use std::cell::RefCell;

use crate::grabs::{ResizeEdge, ResizeState, SurfaceData};
use crate::state::{ClientState, NanaimoState};

impl BufferHandler for NanaimoState {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for NanaimoState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        smithay::wayland::compositor::with_states(surface, |states| {
            states.data_map.insert_if_missing(|| {
                RefCell::new(SurfaceData {
                    resize_state: ResizeState::NotResizing,
                })
            });
        });
    }

    fn commit(&mut self, surface: &WlSurface) {
        tracing::trace!("Surface commit: {:?}", surface);
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);

        let mut resize_state = ResizeState::NotResizing;
        smithay::wayland::compositor::with_states(surface, |states| {
            if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                resize_state = data.borrow().resize_state;
            }
        });

        let window = self
            .space
            .elements()
            .find(|w| {
                w.toplevel()
                    .map(|tl| tl.wl_surface() == surface)
                    .unwrap_or(false)
            })
            .cloned();

        if let Some(window) = window {
            let mut window_loc = self.space.element_location(&window).unwrap();
            let geometry = window.geometry();
            let new_size = geometry.size;

            match resize_state {
                ResizeState::Resizing(data) | ResizeState::WaitingForCommit(data) => {
                    let mut moved = false;
                    match data.edges {
                        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
                            window_loc.x = data.initial_window_location.x
                                + (data.initial_window_size.w - new_size.w);
                            moved = true;
                        }
                        _ => {}
                    }
                    match data.edges {
                        ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
                            window_loc.y = data.initial_window_location.y
                                + (data.initial_window_size.h - new_size.h);
                            moved = true;
                        }
                        _ => {}
                    }
                    if moved {
                        self.space.map_element(window.clone(), window_loc, true);
                    }
                }
                _ => {}
            }

            if let ResizeState::WaitingForCommit(_) = resize_state {
                smithay::wayland::compositor::with_states(surface, |states| {
                    if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                        data.borrow_mut().resize_state = ResizeState::NotResizing;
                    }
                });
            }

            tracing::trace!("Window commit: {:?}", window);
            window.on_commit();
        }
    }
}
