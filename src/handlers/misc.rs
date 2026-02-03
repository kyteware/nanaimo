use smithay::{
    wayland::{
        shm::{ShmState, ShmHandler},
        output::{OutputHandler},
        xdg_activation::{XdgActivationState, XdgActivationHandler},
        fractional_scale::{FractionalScaleHandler, with_fractional_scale},
        compositor::{get_parent, with_states},
        seat::WaylandFocus,
    },
    desktop::utils::surface_primary_scanout_output,
};

use crate::state::{NanaimoState};

impl ShmHandler for NanaimoState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl OutputHandler for NanaimoState {}

impl XdgActivationHandler for NanaimoState {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }
    
    fn request_activation(&mut self, _token: smithay::wayland::xdg_activation::XdgActivationToken, _token_data: smithay::wayland::xdg_activation::XdgActivationTokenData, surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        let window = self.space.elements().find(|w| w.wl_surface().map(|s| *s == surface).unwrap_or(false)).cloned();
        if let Some(window) = window {
            self.space.raise_element(&window, true);
        }
    }
}

impl FractionalScaleHandler for NanaimoState {
    fn new_fractional_scale(&mut self, surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        let mut root = surface.clone();
        while let Some(parent) = get_parent(&root) {
            root = parent;
        }
        
        with_states(&surface, |states| {
            let primary_scanout_output = surface_primary_scanout_output(&surface, states)
                .or_else(|| self.space.outputs().next().cloned());
            if let Some(output) = primary_scanout_output {
                with_fractional_scale(states, |fractional_scale| {
                    fractional_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });
    }
}
