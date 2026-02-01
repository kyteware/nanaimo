use smithay::{
    desktop::Window,
    input::pointer::{GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle, ButtonEvent, RelativeMotionEvent, AxisFrame},
    utils::{Logical, Point, Size},
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
};

use crate::state::NanaimoState;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ResizeEdge: u32 {
        const NONE = 0;
        const TOP = 1;
        const BOTTOM = 2;
        const LEFT = 4;
        const TOP_LEFT = 5;
        const BOTTOM_LEFT = 6;
        const RIGHT = 8;
        const TOP_RIGHT = 9;
        const BOTTOM_RIGHT = 10;
    }
}

impl From<xdg_toplevel::ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(x: xdg_toplevel::ResizeEdge) -> Self {
        Self::from_bits(x as u32).unwrap()
    }
}

pub struct PointerMoveSurfaceGrab {
    pub start_data: GrabStartData<NanaimoState>,
    pub window: Window,
    pub initial_window_location: Point<i32, Logical>,
}

impl PointerGrab<NanaimoState> for PointerMoveSurfaceGrab {
    fn motion(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        _focus: Option<(Window, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        // While the grab is active, no client has pointer focus
        handle.motion(data, None, event);

        let delta = event.location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;

        data.space
            .map_element(self.window.clone(), new_location.to_i32_round(), true);
    }

    fn relative_motion(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        focus: Option<(Window, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().is_empty() {
            // No more buttons are pressed, release the grab.
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn frame(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
    ) {
        handle.frame(data);
    }

    fn start_data(&self) -> &GrabStartData<NanaimoState> {
        &self.start_data
    }
}

pub struct PointerResizeSurfaceGrab {
    pub start_data: GrabStartData<NanaimoState>,
    pub window: Window,
    pub edges: ResizeEdge,
    pub initial_window_location: Point<i32, Logical>,
    pub initial_window_size: Size<i32, Logical>,
    pub last_window_size: Size<i32, Logical>,
}

impl PointerGrab<NanaimoState> for PointerResizeSurfaceGrab {
    fn motion(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        _focus: Option<(Window, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let (mut dx, mut dy) = (event.location - self.start_data.location).into();

        let mut new_window_width = self.initial_window_size.w;
        let mut new_window_height = self.initial_window_size.h;

        let left_right = ResizeEdge::LEFT | ResizeEdge::RIGHT;
        let top_bottom = ResizeEdge::TOP | ResizeEdge::BOTTOM;

        if self.edges.intersects(left_right) {
            if self.edges.intersects(ResizeEdge::LEFT) {
                dx = -dx;
            }
            new_window_width = (self.initial_window_size.w as f64 + dx) as i32;
        }

        if self.edges.intersects(top_bottom) {
            if self.edges.intersects(ResizeEdge::TOP) {
                dy = -dy;
            }
            new_window_height = (self.initial_window_size.h as f64 + dy) as i32;
        }

        new_window_width = new_window_width.max(1);
        new_window_height = new_window_height.max(1);

        self.last_window_size = (new_window_width, new_window_height).into();

        if let Some(toplevel) = self.window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Resizing);
                state.size = Some(self.last_window_size);
            });
            toplevel.send_configure();
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        focus: Option<(Window, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().is_empty() {
            handle.unset_grab(self, data, event.serial, event.time, true);

            if let Some(toplevel) = self.window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });
                toplevel.send_configure();

                // If we resized from top or left, we need to update the window location
                if self.edges.intersects(ResizeEdge::TOP_LEFT) {
                    let geometry = self.window.geometry();
                    let mut location = data.space.element_location(&self.window).unwrap();

                    if self.edges.intersects(ResizeEdge::LEFT) {
                        location.x = self.initial_window_location.x + (self.initial_window_size.w - geometry.size.w);
                    }
                    if self.edges.intersects(ResizeEdge::TOP) {
                        location.y = self.initial_window_location.y + (self.initial_window_size.h - geometry.size.h);
                    }

                    data.space.map_element(self.window.clone(), location, true);
                }
            }
        }
    }

    fn axis(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn frame(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
    ) {
        handle.frame(data);
    }

    fn start_data(&self) -> &GrabStartData<NanaimoState> {
        &self.start_data
    }
}
