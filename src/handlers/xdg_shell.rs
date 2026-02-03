use smithay::{
    desktop::{Window},
    input::{
        Seat,
        pointer::{Focus},
    },
    reexports::wayland_server::protocol::{wl_seat::WlSeat},
    utils::{Serial},
    wayland::{
        shell::xdg::{XdgShellState, XdgShellHandler, ToplevelSurface, PopupSurface, PositionerState, decoration::{XdgDecorationHandler}},
        seat::WaylandFocus,
    },
};

use crate::state::{NanaimoState};

impl XdgShellHandler for NanaimoState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        tracing::info!("New toplevel surface created: {:?}", surface);
        let window = Window::new_wayland_window(surface.clone());
        self.space.map_element(window.clone(), (0, 0), true);
        
        // Configuration
        surface.with_pending_state(|state| {
            state.states.set(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Activated);
        });
        surface.send_configure();
        
        // Focus the new window
        if let Some(surface) = window.wl_surface() {
            let serial = self.serial_counter.next_serial();
            let keyboard = self.seat.get_keyboard().unwrap();
            tracing::info!("Focusing new window: {:?}", window);
            keyboard.set_focus(self, Some(surface.into_owned()), serial);
        }

        self.animation_manager.start_fade_in(&window);
        tracing::info!("Window mapped and animation started");
    }
    
    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // TODO: Handle popups
    }
    
    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        // TODO: Handle popup grabs
    }

    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {
        // TODO: Handle reposition request
    }

    fn move_request(&mut self, surface: ToplevelSurface, wl_seat: WlSeat, serial: Serial) {
        let window = self.space.elements().find(|w| w.toplevel().map(|tl| tl == &surface).unwrap_or(false)).cloned();
        if let Some(window) = window {
            let seat = Seat::from_resource(&wl_seat).unwrap();
            let pointer = seat.get_pointer().unwrap();
            let start_data = pointer.grab_start_data().unwrap();
            let initial_window_location = self.space.element_location(&window).unwrap();

            let grab = crate::grabs::PointerMoveSurfaceGrab {
                start_data,
                window,
                initial_window_location,
            };

            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }

    fn resize_request(&mut self, surface: ToplevelSurface, wl_seat: WlSeat, serial: Serial, edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge) {
        let window = self.space.elements().find(|w| w.toplevel().map(|tl| tl == &surface).unwrap_or(false)).cloned();
        if let Some(window) = window {
            let seat = Seat::from_resource(&wl_seat).unwrap();
            let pointer = seat.get_pointer().unwrap();
            let start_data = pointer.grab_start_data().unwrap();
            let initial_window_location = self.space.element_location(&window).unwrap();
            let initial_window_size = window.geometry().size;

            let grab = crate::grabs::PointerResizeSurfaceGrab {
                start_data,
                window,
                edges: edges.into(),
                initial_window_location,
                initial_window_size,
                last_window_size: initial_window_size,
            };

            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }
}

impl XdgDecorationHandler for NanaimoState {
    fn new_decoration(&mut self, _toplevel: ToplevelSurface) {}
    fn request_mode(&mut self, _toplevel: ToplevelSurface, _mode: smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode) {}
    fn unset_mode(&mut self, _toplevel: ToplevelSurface) {}
}
