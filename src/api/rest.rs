use base64::Engine;
use image::ImageFormat;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;
use tracing::{info, error};

pub struct RestClient {
    client: reqwest::Client,
    base_url: String,
}

impl RestClient {
    pub fn new(host: &str, port: u16) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        let base_url = format!("http://{}:{}", host, port);
        Self { client, base_url }
    }

    pub async fn machine_restart(&self) -> Result<(), reqwest::Error> {
        let url = format!("{}/machine/reboot", self.base_url);
        info!("Sending system reboot request to Moonraker via REST");
        self.client.post(&url).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn machine_shutdown(&self) -> Result<(), reqwest::Error> {
        let url = format!("{}/machine/shutdown", self.base_url);
        info!("Sending system shutdown request to Moonraker via REST");
        self.client.post(&url).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn restart_klipper(&self) -> Result<(), reqwest::Error> {
        let url = format!("{}/printer/restart", self.base_url);
        info!("Sending Klipper restart request to Moonraker via REST");
        self.client.post(&url).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn restart_firmware(&self) -> Result<(), reqwest::Error> {
        let url = format!("{}/printer/firmware_restart", self.base_url);
        info!("Sending Klipper firmware restart request to Moonraker via REST");
        self.client.post(&url).send().await?.error_for_status()?;
        Ok(())
    }
}

pub fn extract_thumbnail_from_gcode<P: AsRef<Path>>(file_path: P) -> Option<Image> {
    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    let mut in_thumbnail = false;
    let mut base64_accumulator = String::new();

    for (line_idx, line_res) in reader.lines().enumerate() {
        if line_idx > 5000 {
            break;
        }

        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };

        let trimmed = line.trim();

        if !in_thumbnail {
            if trimmed.starts_with("; thumbnail begin") || (trimmed.starts_with("; thumbnail_") && trimmed.contains("begin")) {
                in_thumbnail = true;
                base64_accumulator.clear();
            }
        } else {
            if trimmed.starts_with("; thumbnail end") || (trimmed.starts_with("; thumbnail_") && trimmed.contains("end")) {
                if let Ok(png_bytes) = base64::prelude::BASE64_STANDARD.decode(base64_accumulator.trim()) {
                    if let Ok(img) = image::load_from_memory_with_format(&png_bytes, ImageFormat::Png) {
                        let rgba = img.to_rgba8();
                        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                            rgba.as_raw(),
                            rgba.width(),
                            rgba.height(),
                        );
                        return Some(Image::from_rgba8(buffer));
                    }
                }
                break;
            }

            if trimmed.starts_with(';') {
                let payload = trimmed[1..].trim();
                base64_accumulator.push_str(payload);
            }
        }
    }

    None
}
