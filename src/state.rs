use smithay::{
    delegate_compositor, delegate_output, delegate_seat,
    delegate_shm, delegate_viewporter, delegate_xdg_shell,
    delegate_layer_shell,
    desktop::{Space, Window, WindowSurfaceType},
    input::{
        Seat, SeatState,
        pointer::{PointerHandle, MotionEvent, ButtonEvent, AxisFrame}, 
        keyboard::{FilterResult, Keycode},
    },
    reexports::{
        calloop::LoopHandle,
        wayland_server::{Display, DisplayHandle, backend::{ClientData, ClientId, DisconnectReason}, protocol::wl_surface::WlSurface},
    },
    utils::{Point, Logical, Serial},
    backend::input::KeyState,
    wayland::{
        compositor::{CompositorState, CompositorClientState},
        viewporter::ViewporterState,
        xdg_activation::{XdgActivationState},
        fractional_scale::{FractionalScaleManagerState},
        selection::data_device::{DataDeviceState},
        shell::{
            xdg::{XdgShellState},
            wlr_layer::{WlrLayerShellState},
        },
        shm::{ShmState},
        seat::WaylandFocus,
    },
};
use smithay::backend::input::{Event, PointerAxisEvent};

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
    pub xdg_decoration_state: smithay::wayland::shell::xdg::decoration::XdgDecorationState,
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
    pub fn new(display: &Display<NanaimoState>, _loop_handle: LoopHandle<'static, NanaimoState>) -> Self {
        tracing::info!("Initializing NanaimoState...");
        let dh = display.handle();
        
        let compositor_state = CompositorState::new::<Self>(&dh);
        let viewporter_state = ViewporterState::new::<Self>(&dh);
        let xdg_activation_state = XdgActivationState::new::<Self>(&dh);
        let fractional_scale_manager_state = FractionalScaleManagerState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_decoration_state = smithay::wayland::shell::xdg::decoration::XdgDecorationState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let _output_manager_state = smithay::wayland::output::OutputManagerState::new_with_xdg_output::<Self>(&dh);
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
        
        if let Some((ref surface, _)) = under {
             tracing::trace!("Pointer over surface: {:?}", surface);
        }

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
        tracing::debug!("Pointer button: {:?} state: {:?} at {:?}", button, state, self.pointer.current_location());
        
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

    pub fn on_pointer_axis<B: smithay::backend::input::InputBackend>(&mut self, event: B::PointerAxisEvent) {
        let mut frame = AxisFrame::new(event.time_msec());
        if let Some(v) = event.amount_v120(smithay::backend::input::Axis::Vertical) {
            frame = frame.v120(smithay::backend::input::Axis::Vertical, v as i32);
        } else if let Some(v) = event.amount(smithay::backend::input::Axis::Vertical) {
            frame = frame.value(smithay::backend::input::Axis::Vertical, v);
        }
        if let Some(v) = event.amount_v120(smithay::backend::input::Axis::Horizontal) {
            frame = frame.v120(smithay::backend::input::Axis::Horizontal, v as i32);
        } else if let Some(v) = event.amount(smithay::backend::input::Axis::Horizontal) {
            frame = frame.value(smithay::backend::input::Axis::Horizontal, v);
        }
        
        let pointer = self.pointer.clone();
        pointer.axis(self, frame);
        pointer.frame(self);
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
        let under = self.space.element_under(pos).map(|(w, p)| (w.clone(), p));
        tracing::debug!("Updating keyboard focus, pointer at {:?}, found window: {:?}", pos, under.as_ref().map(|(w, _)| w));

        if let Some((window, _)) = under {
            self.space.raise_element(&window, true);
            let keyboard = self.seat.get_keyboard().unwrap();
            
            if keyboard.current_focus().as_ref().map(|f| f.wl_surface().as_deref() == window.wl_surface().as_deref()).unwrap_or(false) {
                tracing::debug!("Window already focused");
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
                tracing::info!("Setting keyboard focus to window: {:?}", window);
                keyboard.set_focus(self, Some(surface.into_owned()), serial);
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
