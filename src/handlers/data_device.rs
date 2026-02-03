use smithay::{
    wayland::{
        selection::{SelectionHandler},
        selection::data_device::{DataDeviceState, DataDeviceHandler, WaylandDndGrabHandler},
    },
};

use crate::state::{NanaimoState};

impl SelectionHandler for NanaimoState {
    type SelectionUserData = ();
}

impl WaylandDndGrabHandler for NanaimoState {}

impl DataDeviceHandler for NanaimoState {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device_state
    }
}
