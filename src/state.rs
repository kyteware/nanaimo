use smithay::{
    delegate_compositor, delegate_output, delegate_seat,
    delegate_shm, delegate_viewporter, delegate_xdg_shell,
    delegate_layer_shell,
    desktop::{Space, Window, WindowSurfaceType},
    input::{
        Seat, SeatState, SeatHandler, 
        pointer::{CursorImageStatus, PointerHandle, MotionEvent, ButtonEvent, Focus, AxisFrame}, 
        keyboard::{FilterResult, Keycode},
    },
    reexports::{
        calloop::LoopHandle,
        wayland_server::{Display, DisplayHandle, backend::{ClientData, ClientId, DisconnectReason}, protocol::wl_buffer::WlBuffer, protocol::wl_output::WlOutput},
    },
    utils::{Point, Logical, Serial},
    backend::input::KeyState,
    wayland::{
        compositor::{CompositorState, CompositorClientState, CompositorHandler},
        viewporter::ViewporterState,
        xdg_activation::{XdgActivationState, XdgActivationHandler},
        fractional_scale::{FractionalScaleManagerState, FractionalScaleHandler},
        selection::data_device::{DataDeviceState, DataDeviceHandler, WaylandDndGrabHandler},
        output::{OutputManagerState, OutputHandler},
        shell::{
            xdg::{XdgShellState, XdgShellHandler, ToplevelSurface, PopupSurface, PositionerState, decoration::{XdgDecorationState, XdgDecorationHandler}},
            wlr_layer::{WlrLayerShellState, WlrLayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        },
        shm::{ShmState, ShmHandler},
        seat::WaylandFocus,
        buffer::BufferHandler,
    },
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::fractional_scale::with_fractional_scale;
use smithay::desktop::utils::surface_primary_scanout_output;
use smithay::wayland::compositor::get_parent;
use smithay::wayland::compositor::with_states;
use crate::animations::AnimationManager;

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, client_id: ClientId) {
        tracing::info!("Client {:?} initialized", client_id);
    }
    fn disconnected(&self, client_id: ClientId, _reason: DisconnectReason) {
        tracing::info!("Client {:?} disconnected", client_id);
    }
}

pub struct NanaimoState {
    pub space: Space<Window>,
    pub compositor_state: CompositorState,
    pub viewporter_state: ViewporterState,
    pub xdg_activation_state: XdgActivationState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,
    pub xdg_shell_state: XdgShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub layer_shell_state: WlrLayerShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<NanaimoState>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<NanaimoState>,
    pub pointer: PointerHandle<NanaimoState>,
    pub animation_manager: AnimationManager,
    
    pub display_handle: DisplayHandle,
    pub serial_counter: smithay::utils::SerialCounter,
}

impl NanaimoState {
    pub fn new(display: &Display<NanaimoState>, loop_handle: LoopHandle<'static, NanaimoState>) -> Self {
        tracing::info!("Initializing NanaimoState...");
        let dh = display.handle();
        
        let compositor_state = CompositorState::new::<Self>(&dh);
        let viewporter_state = ViewporterState::new::<Self>(&dh);
        let xdg_activation_state = XdgActivationState::new::<Self>(&dh);
        let fractional_scale_manager_state = FractionalScaleManagerState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&dh, "nanaimo");
        
        let xkb_config = smithay::input::keyboard::XkbConfig {
            layout: "us",
            ..Default::default()
        };
        seat.add_keyboard(xkb_config, 200, 25).expect("Failed to add keyboard");
        let pointer = seat.add_pointer();
        
        let data_device_state = DataDeviceState::new::<Self>(&dh);

        Self {
            space: Space::default(),
            compositor_state,
            viewporter_state,
            xdg_activation_state,
            fractional_scale_manager_state,
            xdg_shell_state,
            xdg_decoration_state,
            layer_shell_state,
            shm_state,
            seat_state,
            data_device_state,
            seat,
            pointer,
            animation_manager: AnimationManager::new(),
            display_handle: dh,
            serial_counter: smithay::utils::SerialCounter::default(),
        }
    }
    
    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space.element_under(pos).and_then(|(window, loc)| {
            window.surface_under(pos - loc.to_f64(), WindowSurfaceType::ALL)
                .map(|(surface, surf_loc)| (surface, surf_loc.to_f64() + loc.to_f64()))
        })
    }
    
    pub fn on_pointer_move_absolute(&mut self, pos: Point<f64, Logical>, time: u32) {
        let serial = self.serial_counter.next_serial();
        let under = self.surface_under(pos);
        let pointer = self.pointer.clone();
        pointer.motion(
            self,
            under,
            &MotionEvent {
                location: pos,
                serial,
                time,
            },
        );
        pointer.frame(self);
    }
    
    pub fn on_pointer_button(&mut self, button: u32, state: smithay::backend::input::ButtonState, time: u32) {
        let serial = self.serial_counter.next_serial();
        if state == smithay::backend::input::ButtonState::Pressed {
            self.update_keyboard_focus(serial);
        }
        
        let pointer = self.pointer.clone();
        pointer.button(
            self,
            &ButtonEvent {
                button,
                state,
                serial,
                time,
            },
        );
        pointer.frame(self);
    }

    pub fn on_pointer_axis(&mut self, event: smithay::backend::input::PointerAxisEvent) {
        let mut frame = AxisFrame::new(event.time_msec());
        if let Some(v) = event.amount_v120(smithay::backend::input::Axis::Vertical) {
            frame = frame.v120(smithay::backend::input::Axis::Vertical, v);
        } else if let Some(v) = event.amount(smithay::backend::input::Axis::Vertical) {
            frame = frame.value(smithay::backend::input::Axis::Vertical, v);
        }
        if let Some(v) = event.amount_v120(smithay::backend::input::Axis::Horizontal) {
            frame = frame.v120(smithay::backend::input::Axis::Horizontal, v);
        } else if let Some(v) = event.amount(smithay::backend::input::Axis::Horizontal) {
            frame = frame.value(smithay::backend::input::Axis::Horizontal, v);
        }
        
        self.pointer.axis(self, frame);
        self.pointer.frame(self);
    }
    
    pub fn on_keyboard_key(&mut self, keycode: Keycode, state: KeyState, time: u32) {
        let serial = self.serial_counter.next_serial();
        let keyboard = self.seat.get_keyboard().unwrap();
        let focus = keyboard.current_focus();
        tracing::debug!("Keyboard key: {:?} state: {:?} current_focus: {:?}", keycode, state, focus.as_ref().map(|f| f.wl_surface()));
        
        keyboard.input::<(), _>(
            self,
            keycode,
            state,
            serial,
            time,
            |_, _, _| {
                tracing::debug!("Forwarding key event to client");
                FilterResult::Forward
            },
        );
    }
    
    fn update_keyboard_focus(&mut self, serial: Serial) {
        let pos = self.pointer.current_location();
        if let Some((window, _)) = self.space.element_under(pos).map(|(w, p)| (w.clone(), p)) {
            self.space.raise_element(&window, true);
            let keyboard = self.seat.get_keyboard().unwrap();
            
            if keyboard.current_focus().as_ref().map(|f| f.wl_surface().as_deref() == window.wl_surface().as_deref()).unwrap_or(false) {
                return;
            }

            // Deactivate other windows
            for other in self.space.elements() {
                if other != &window {
                    if let Some(toplevel) = other.toplevel() {
                        toplevel.with_pending_state(|state| {
                            state.states.unset(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Activated);
                        });
                        toplevel.send_configure();
                    }
                }
            }

            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.set(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Activated);
                });
                toplevel.send_configure();
            }

            if let Some(surface) = window.wl_surface() {
                tracing::info!("Setting keyboard focus to window: {:?} at logical {:?} (converted from physical via main.rs)", window, pos);
                keyboard.set_focus(self, Some(surface.into_owned()), serial);
                
                // Modifier sync is handled by set_focus in Smithay, so we don't need a manual call here.
            }
        }
    }
}

// Delegate implementations
delegate_compositor!(NanaimoState);
delegate_shm!(NanaimoState);
delegate_seat!(NanaimoState);
delegate_output!(NanaimoState);
delegate_xdg_shell!(NanaimoState);
delegate_layer_shell!(NanaimoState);
delegate_viewporter!(NanaimoState);
smithay::delegate_xdg_activation!(NanaimoState);
smithay::delegate_fractional_scale!(NanaimoState);
smithay::delegate_xdg_decoration!(NanaimoState);
smithay::delegate_data_device!(NanaimoState);

impl BufferHandler for NanaimoState {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for NanaimoState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
    
    fn client_compositor_state<'a>(&self, client: &'a smithay::reexports::wayland_server::Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        tracing::debug!("Surface commit: {:?}", surface);
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
        if let Some(window) = self.space.elements().find(|w| w.toplevel().map(|tl| tl.wl_surface() == surface).unwrap_or(false)) {
             tracing::debug!("Window commit: {:?}", window);
             window.on_commit();
        }
    }
}

impl ShmHandler for NanaimoState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

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

impl OutputHandler for NanaimoState {}

impl XdgShellHandler for NanaimoState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        tracing::info!("New toplevel surface created: {:?}", surface);
        let window = Window::new_wayland_window(surface.clone());
        self.space.map_element(window.clone(), (0, 0), true);
        
        // Configuration is often better triggered on first commit or explicitly if we know the size
        surface.with_pending_state(|state| {
            state.states.set(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Activated);
        });
        surface.send_configure();
        
        self.animation_manager.start_fade_in(&window);
        tracing::info!("Window mapped and animation started");
    }
    
    fn new_popup(&mut self, _surface: PopupSurface, _positioner: smithay::wayland::shell::xdg::PositionerState) {
        // TODO: Handle popups
    }
    
    fn grab(&mut self, _surface: PopupSurface, _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat, _serial: smithay::utils::Serial) {
        // TODO: Handle popup grabs
    }

    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {
        // TODO: Handle reposition request
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: Seat<Self>, serial: Serial) {
        let window = self.space.elements().find(|w| w.toplevel().map(|tl| tl == &surface).unwrap_or(false)).cloned();
        if let Some(window) = window {
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

    fn resize_request(&mut self, surface: ToplevelSurface, seat: Seat<Self>, serial: Serial, edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge) {
        let window = self.space.elements().find(|w| w.toplevel().map(|tl| tl == &surface).unwrap_or(false)).cloned();
        if let Some(window) = window {
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

impl SelectionHandler for NanaimoState {
    type SelectionUserData = ();
}

impl WaylandDndGrabHandler for NanaimoState {}

impl DataDeviceHandler for NanaimoState {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device_state
    }
}
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

// Viewporter doesn't need a formal Handler trait in this version, delegate_viewporter! handles it via state access.
// If it fails, I'll add it back with the correct name.
