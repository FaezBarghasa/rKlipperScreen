use evdev::{Device, InputEventKind, AbsoluteAxisType, Key};
use slint::platform::{WindowEvent, PointerEventButton};
use slint::LogicalPosition;
use slint::ComponentHandle;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

pub async fn start_touch_router(app_weak: slint::Weak<crate::MainApp>) {
    info!("Starting evdev touch input router for KMS mode");
    
    let mut device_opt = None;
    for _ in 0..10 {
        match Device::open("/dev/input/event0") {
            Ok(device) => {
                device_opt = Some(device);
                break;
            }
            Err(e) => {
                warn!("Failed to open /dev/input/event0, retrying: {}", e);
                sleep(Duration::from_millis(500)).await;
            }
        }
    }

    let device = match device_opt {
        Some(d) => d,
        None => {
            error!("Could not open touch device at /dev/input/event0");
            return;
        }
    };

    info!("Touch device opened: {:?}", device.name());

    let mut stream = match device.into_event_stream() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create event stream: {}", e);
            return;
        }
    };

    let mut current_x = 0.0;
    let mut current_y = 0.0;

    tokio::spawn(async move {
        while let Ok(event) = stream.next_event().await {
            match event.kind() {
                InputEventKind::AbsAxis(axis) => {
                    match axis {
                        AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_MT_POSITION_X => {
                            current_x = scale_coordinate(event.value() as f32, 4096.0, 480.0);
                        }
                        AbsoluteAxisType::ABS_Y | AbsoluteAxisType::ABS_MT_POSITION_Y => {
                            current_y = scale_coordinate(event.value() as f32, 4096.0, 320.0);
                        }
                        _ => {}
                    }

                    
                    // Dispatch move event
                    let _ = slint::invoke_from_event_loop({
                        let app_weak = app_weak.clone();
                        let pos = LogicalPosition::new(current_x, current_y);
                        move || {
                            if let Some(app) = app_weak.upgrade() {
                                app.window().dispatch_event(WindowEvent::PointerMoved { position: pos });
                            }
                        }
                    });
                }
                InputEventKind::Key(key) => {
                    if key == Key::BTN_TOUCH {
                        let is_pressed = event.value() != 0;
                        let _ = slint::invoke_from_event_loop({
                            let app_weak = app_weak.clone();
                            let pos = LogicalPosition::new(current_x, current_y);
                            move || {
                                if let Some(app) = app_weak.upgrade() {
                                    if is_pressed {
                                        app.window().dispatch_event(WindowEvent::PointerPressed {
                                            position: pos,
                                            button: PointerEventButton::Left,
                                        });
                                    } else {
                                        app.window().dispatch_event(WindowEvent::PointerReleased {
                                            position: pos,
                                            button: PointerEventButton::Left,
                                        });
                                    }
                                }
                            }
                        });
                    }
                }
                _ => {}
            }
        }
    });
}

pub fn scale_coordinate(raw: f32, max_raw: f32, max_scaled: f32) -> f32 {
    if max_raw == 0.0 {
        return 0.0;
    }
    (raw / max_raw) * max_scaled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_coordinate_origin() {
        assert_eq!(scale_coordinate(0.0, 4096.0, 480.0), 0.0);
    }

    #[test]
    fn test_scale_coordinate_max() {
        assert_eq!(scale_coordinate(4096.0, 4096.0, 480.0), 480.0);
    }

    #[test]
    fn test_scale_coordinate_half() {
        assert_eq!(scale_coordinate(2048.0, 4096.0, 320.0), 160.0);
    }

    #[test]
    fn test_scale_coordinate_zero_max_raw() {
        assert_eq!(scale_coordinate(100.0, 0.0, 480.0), 0.0);
    }
}
