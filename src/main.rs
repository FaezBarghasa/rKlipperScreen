use std::env;

mod api;
mod system;

// Include the auto-generated code from build.rs containing our RKlipperScreen UI
slint::include_modules!();

/// Phase 1: Dynamic Graphics Backend Autodetection
/// Checks system environments to intelligently fallback to DRM/KMS if X11 or Wayland aren't active.
fn configure_display_backend() {
    if env::var("WAYLAND_DISPLAY").is_ok() {
        env::set_var("SLINT_BACKEND", "winit");
    } else if env::var("DISPLAY").is_ok() {
        env::set_var("SLINT_BACKEND", "winit");
    } else {
        // Bound to bare metal. Run without seats directly on /dev/dri/card0
        env::set_var("SLINT_BACKEND", "linuxkms-noseat");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    configure_display_backend();
    
    let ui = RKlipperScreen::new()?;
    let ui_handle = ui.as_weak();

    // Phase 3: Spin up the WS Async Connection
    tokio::spawn(async move {
        api::websocket::connect_to_moonraker(ui_handle).await;
    });

    // Phase 2: Start raw touch coordinate processing and injection loop
    std::thread::spawn(|| {
        system::touch::run_input_listener();
    });

    ui.run()?;
    Ok(())
}