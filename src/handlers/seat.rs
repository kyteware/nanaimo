use smithay::{
    input::{
        Seat, SeatState, SeatHandler,
        pointer::{CursorImageStatus},
    },
    reexports::wayland_server::protocol::{wl_surface::WlSurface},
    wayland::tablet_manager::TabletSeatHandler,
};

use crate::state::{NanaimoState};

impl SeatHandler for NanaimoState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<NanaimoState> {
        &mut self.seat_state
    }
    
    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        tracing::trace!("Cursor image changed: {:?}", image);
        // TODO: Actually render the cursor based on image status (surface or named)
    }
    
    fn focus_changed(&mut self, _seat: &Seat<Self>, _focus: Option<&Self::KeyboardFocus>) {
        tracing::trace!("Focus changed: {:?}", _focus);
    }
}

impl TabletSeatHandler for NanaimoState {
    fn tablet_tool_image(&mut self, _tool: &smithay::backend::input::TabletToolDescriptor, _image: CursorImageStatus) {
        // TODO: Handle tablet tool images
    }
}
