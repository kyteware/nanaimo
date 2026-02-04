use smithay::{
    desktop::Window,
    input::pointer::{
        AxisFrame, ButtonEvent, CursorIcon, CursorImageStatus, GestureHoldBeginEvent,
        GestureHoldEndEvent, GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
        GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData,
        MotionEvent, PointerGrab, PointerInnerHandle, RelativeMotionEvent,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Serial, Size},
    wayland::{compositor::with_states, seat::WaylandFocus},
};
use std::cell::RefCell;

use crate::state::NanaimoState;

pub use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;

/// Information about the resize operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ResizeData {
    /// The edges the surface is being resized with.
    pub edges: ResizeEdge,
    /// The initial window location.
    pub initial_window_location: Point<i32, Logical>,
    /// The initial window size (geometry width and height).
    pub initial_window_size: Size<i32, Logical>,
}

/// State of the resize operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ResizeState {
    /// The surface is not being resized.
    #[default]
    NotResizing,
    /// The surface is currently being resized.
    Resizing(ResizeData),
    /// The resize has finished, and the surface needs to ack the final configure.
    WaitingForFinalAck(ResizeData, Serial),
    /// The resize has finished, and the surface needs to commit its final state.
    WaitingForCommit(ResizeData),
}

pub struct SurfaceData {
    pub resize_state: ResizeState,
}

pub fn cursor_icon_for_edge(edge: ResizeEdge) -> CursorIcon {
    match edge {
        ResizeEdge::Top => CursorIcon::NResize,
        ResizeEdge::Bottom => CursorIcon::SResize,
        ResizeEdge::Left => CursorIcon::WResize,
        ResizeEdge::Right => CursorIcon::EResize,
        ResizeEdge::TopLeft => CursorIcon::NwResize,
        ResizeEdge::TopRight => CursorIcon::NeResize,
        ResizeEdge::BottomLeft => CursorIcon::SwResize,
        ResizeEdge::BottomRight => CursorIcon::SeResize,
        _ => CursorIcon::Default,
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
        _focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        // While the grab is active, no client has pointer focus
        handle.motion(data, None, event);

        // wish we could do this, but some clients don't send a grab for a few pixels, so it feels off
        // data.cursor_status = CursorImageStatus::Named(CursorIcon::Grabbing);

        let delta = event.location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;

        data.space
            .map_element(self.window.clone(), new_location.to_i32_round(), true);
    }

    fn relative_motion(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        focus: Option<(WlSurface, Point<f64, Logical>)>,
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

    fn gesture_swipe_begin(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<NanaimoState> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut NanaimoState) {}
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
        _focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);
        data.cursor_status = CursorImageStatus::Named(cursor_icon_for_edge(self.edges));

        let (mut dx, mut dy) = (event.location - self.start_data.location).into();

        let mut new_window_width = self.initial_window_size.w;
        let mut new_window_height = self.initial_window_size.h;

        match self.edges {
            ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
                dx = -dx;
                new_window_width = (self.initial_window_size.w as f64 + dx) as i32;
            }
            ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => {
                new_window_width = (self.initial_window_size.w as f64 + dx) as i32;
            }
            _ => {}
        }

        match self.edges {
            ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
                dy = -dy;
                new_window_height = (self.initial_window_size.h as f64 + dy) as i32;
            }
            ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
                new_window_height = (self.initial_window_size.h as f64 + dy) as i32;
            }
            _ => {}
        }

        new_window_width = new_window_width.max(1);
        new_window_height = new_window_height.max(1);

        self.last_window_size = (new_window_width, new_window_height).into();

        if let Some(surface) = self.window.wl_surface() {
            with_states(&surface, |states| {
                let mut data = states
                    .data_map
                    .get::<RefCell<SurfaceData>>()
                    .unwrap()
                    .borrow_mut();
                data.resize_state = ResizeState::Resizing(ResizeData {
                    edges: self.edges,
                    initial_window_location: self.initial_window_location,
                    initial_window_size: self.initial_window_size,
                });
            });
        }

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
        focus: Option<(WlSurface, Point<f64, Logical>)>,
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

                if let Some(surface) = self.window.wl_surface() {
                    with_states(&surface, |states| {
                        let mut data = states
                            .data_map
                            .get::<RefCell<SurfaceData>>()
                            .unwrap()
                            .borrow_mut();
                        if let ResizeState::Resizing(resize_data) = data.resize_state {
                            data.resize_state =
                                ResizeState::WaitingForFinalAck(resize_data, event.serial);
                        }
                    });
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

    fn gesture_swipe_begin(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut NanaimoState,
        handle: &mut PointerInnerHandle<'_, NanaimoState>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<NanaimoState> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut NanaimoState) {}
}
