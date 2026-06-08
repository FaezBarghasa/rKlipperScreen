use std::fs;
use std::path::Path;
use std::process::Command;
use tracing::{error, info, warn};

pub struct PowerController {
    backlight_path: Option<String>,
}

impl PowerController {
    pub fn new() -> Self {
        // Look for common backlight nodes on SBCs (like Raspberry Pi)
        let paths = vec![
            "/sys/class/backlight/rpi_backlight/brightness",
            "/sys/class/backlight/backlight/brightness",
        ];

        let mut found_path = None;
        for p in paths {
            if Path::new(p).exists() {
                info!("Found sysfs backlight controller at {}", p);
                found_path = Some(p.to_string());
                break;
            }
        }

        Self {
            backlight_path: found_path,
        }
    }

    pub fn set_screen_power(&self, turn_on: bool) {
        if let Some(ref path) = self.backlight_path {
            let val = if turn_on { "255" } else { "0" };
            info!("Writing backlight brightness {} to {}", val, path);
            if let Err(err) = fs::write(path, val) {
                error!("Failed to write brightness to sysfs path {}: {}", path, err);
            }
        } else {
            // X11 dpms fallback
            let state = if turn_on { "on" } else { "off" };
            info!("Using X11 DPMS fallback to turn screen {}", state);
            let mut cmd = Command::new("xset");
            cmd.arg("dpms").arg("force").arg(state);

            // Slint often runs inside an X11/Wayland context; set DISPLAY variable to default if not set
            if std::env::var("DISPLAY").is_err() {
                cmd.env("DISPLAY", ":0");
            }

            match cmd.status() {
                Ok(status) => {
                    if !status.success() {
                        warn!("xset dpms returned non-zero exit status: {:?}", status);
                    }
                }
                Err(err) => {
                    warn!("Failed to execute xset command for DPMS power: {}", err);
                }
            }
        }
    }
}
