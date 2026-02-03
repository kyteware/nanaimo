use smithay::{
    input::{
        Seat, SeatState, SeatHandler,
        pointer::{CursorImageStatus},
    },
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
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
        self.cursor_status = image;
    }
    
    fn focus_changed(&mut self, seat: &Seat<Self>, focus: Option<&Self::KeyboardFocus>) {
        tracing::debug!("Focus changed: {:?}", focus);
        
        let dh = &self.display_handle;
        let client = focus.and_then(|s| dh.get_client(s.id()).ok());
        
        smithay::wayland::selection::data_device::set_data_device_focus(dh, seat, client.clone());
        smithay::wayland::selection::primary_selection::set_primary_focus(dh, seat, client);
    }
}

impl TabletSeatHandler for NanaimoState {
    fn tablet_tool_image(&mut self, _tool: &smithay::backend::input::TabletToolDescriptor, _image: CursorImageStatus) {
        // TODO: Handle tablet tool images
    }
}
