use smithay::{
    input::{
        Seat, SeatState, SeatHandler,
        pointer::{CursorImageStatus},
    },
    reexports::wayland_server::protocol::{wl_surface::WlSurface},
};

use crate::state::{NanaimoState};

impl SeatHandler for NanaimoState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<NanaimoState> {
        &mut self.seat_state
    }
    
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}
    fn focus_changed(&mut self, _seat: &Seat<Self>, _focus: Option<&Self::KeyboardFocus>) {
        tracing::debug!("Focus changed: {:?}", _focus);
    }
}
