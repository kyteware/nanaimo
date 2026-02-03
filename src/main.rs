use smithay::{
    backend::{
        winit::{self, WinitEvent},
        renderer::{
            damage::OutputDamageTracker,
            gles::GlesRenderer,
            ImportMemWl,
            ImportEgl,
        },
    },
    reexports::{
        calloop::EventLoop,
        wayland_server::Display,
    },
    backend::input::{AbsolutePositionEvent, Event, PointerButtonEvent, KeyboardKeyEvent},
};
use std::time::Duration;

mod state;
mod animations;
mod render;
mod grabs;
mod handlers;
use state::{NanaimoState, ClientState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Logging
    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info,nanaimo=info,smithay=warn"); }
    }
    tracing_subscriber::fmt::init();

    // 2. Event Loop
    let mut event_loop = EventLoop::try_new()?;
    let loop_handle = event_loop.handle();

    // 3. Display
    let mut display: Display<NanaimoState> = Display::new()?;
    let display_handle = display.handle();

    let listening_socket = smithay::wayland::socket::ListeningSocketSource::new_auto()?;
    let socket_name = listening_socket.socket_name().to_string_lossy().into_owned();
    tracing::info!("Listening on WAYLAND_DISPLAY={}", socket_name);

    loop_handle.insert_source(listening_socket, move |client_stream, _, state: &mut NanaimoState| {
        if let Err(err) = state.display_handle.insert_client(client_stream, std::sync::Arc::new(ClientState::default())) {
            tracing::warn!("Error adding client: {}", err);
        }
    })?;
    
    // 4. State
    let mut state = NanaimoState::new(&display, loop_handle.clone());

    // 5. Winit Backend
    let (mut backend, mut winit) = winit::init::<GlesRenderer>()?;
    
    // Update SHM formats
    let shm_formats: Vec<_> = backend.renderer().shm_formats().collect();
    tracing::info!("Supported SHM formats: {:?}", shm_formats);
    state.shm_state.update_formats(shm_formats);
    
    // Enable EGL hardware acceleration for clients
    let _ = backend.renderer().bind_wl_display(&display_handle);
    
    let mode = smithay::output::Mode {
        size: (1280, 800).into(),
        refresh: 60_000,
    };

    let output = smithay::output::Output::new(
        "winit".to_string(),
        smithay::output::PhysicalProperties {
            size: (0, 0).into(),
            subpixel: smithay::output::Subpixel::Unknown,
            make: "Smithay".into(),
            model: "Winit".into(),
            serial_number: "unknown".into(),
        },
    );
    let _global = output.create_global::<NanaimoState>(&display_handle);
    output.change_current_state(Some(mode), Some(smithay::utils::Transform::Flipped180), None, Some((0, 0).into()));
    output.set_preferred(mode);
    
    // Map output to space
    state.space.map_output(&output, (0, 0));

    // Damage Tracker
    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    // 6. Run
    tracing::info!("Starting Nanaimo Compositor...");

    // Insert winit backend into event loop
    loop_handle.insert_source(
        smithay::reexports::calloop::timer::Timer::immediate(),
        move |_, _, state| {
            // Trigger initial render
            state.space.refresh();
            smithay::reexports::calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(16))
        },
    )?;
    
    loop {
        // Dispatch calloop
        let result = event_loop.dispatch(Some(Duration::from_millis(1)), &mut state);
        if result.is_err() {
            tracing::error!("Event loop error: {:?}", result.err());
            break;
        }
        
        display.dispatch_clients(&mut state).expect("Failed to dispatch clients");
        display.flush_clients().unwrap();
        
        // Dispatch winit events using the 'winit' handler, NOT backend
        let _ = winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                let mode = smithay::output::Mode {
                    size,
                    refresh: 60_000,
                };
                output.change_current_state(Some(mode), None, None, None);
                state.space.map_output(&output, (0, 0));
            }
            WinitEvent::Input(event) => {
                use smithay::backend::input::InputEvent;
                match event {
                    InputEvent::PointerMotionAbsolute { event } => {
                        let output_geo = state.space.output_geometry(&output).unwrap();
                        let final_pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                        
                        tracing::trace!("Pointer move: logical={:?}", final_pos);
                        state.on_pointer_move_absolute(final_pos, event.time_msec());
                    }
                    InputEvent::PointerButton { event } => {
                        state.on_pointer_button(event.button_code(), event.state(), event.time_msec());
                    }
                    InputEvent::Keyboard { event } => {
                        state.on_keyboard_key(event.key_code(), event.state(), event.time_msec());
                    }
                    InputEvent::PointerAxis { event } => {
                        state.on_pointer_axis::<winit::WinitInput>(event);
                    }
                    _ => (),
                }
            }
            WinitEvent::CloseRequested => {
                // Shutdown
                std::process::exit(0);
            }
            _ => (),
        });
        
        // Render
        state.animation_manager.tick();
        
        let render_res = backend.bind().map(|(renderer, mut framebuffer)| {
            render::render_output(
                &output,
                &state.space,
                renderer,
                &mut framebuffer,
                &mut damage_tracker,
                0,
            )
        });
        
        match render_res {
            Ok(Ok(render_result)) => {
                    if let Some(damage) = render_result.damage {
                        if !damage.is_empty() {
                            if let Err(err) = backend.submit(Some(damage)) {
                                tracing::warn!("Submit failed: {}", err);
                            }
                        } else {
                            let _ = backend.submit(None);
                        }
                    } else {
                         let _ = backend.submit(None);
                    }
                    
                    // Send frame callbacks to clients
                    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                    for window in state.space.elements() {
                        window.send_frame(&output, Duration::from_millis(time), Some(Duration::from_millis(16)), |_, _| None);
                    }
                }
            Ok(Err(err)) => {
                tracing::error!("Render error: {}", err);
            }
            Err(err) => {
                tracing::error!("Bind error: {}", err);
            }
        }
        
        state.space.refresh();
    }
    
    Ok(())
}
