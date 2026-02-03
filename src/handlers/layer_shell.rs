use smithay::{
    reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    wayland::{
        shell::wlr_layer::{WlrLayerShellState, WlrLayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
};

use crate::state::{NanaimoState};

impl WlrLayerShellHandler for NanaimoState {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        _surface: LayerSurface,
        _output: Option<WlOutput>,
        _layer: smithay::wayland::shell::wlr_layer::Layer,
        _namespace: String,
    ) {
    }

    fn ack_configure(&mut self, _surface: WlSurface, _configure: LayerSurfaceConfigure) {}
}
