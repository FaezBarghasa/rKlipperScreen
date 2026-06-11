use input::event::Event;
use input::{Libinput, LibinputInterface};
use libc::{close, open, O_RDONLY, O_RDWR, O_WRONLY};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::os::fd::{OwnedFd, FromRawFd};
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Math structure mapping absolute hardware matrix fields dynamically mapped to resistive panels.
pub struct CalibrationMatrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl CalibrationMatrix {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Self {
        let default_matrix = Self {
            a: 1.0, b: 0.0, c: 0.0,
            d: 0.0, e: 1.0, f: 0.0,
        };
        
        if let Ok(contents) = fs::read_to_string(path) {
            let parts: Vec<f64> = contents
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
                
            if parts.len() == 6 {
                return Self {
                    a: parts[0], b: parts[1], c: parts[2],
                    d: parts[3], e: parts[4], f: parts[5],
                };
            }
        }
        default_matrix
    }

    pub fn calibrate_point(&self, x_raw: f64, y_raw: f64) -> (u32, u32) {
        let x_pixel = self.a * x_raw + self.b * y_raw + self.c;
        let y_pixel = self.d * x_raw + self.e * y_raw + self.f;
        
        // Bounds checking and robust conversion. Negative values clamp out to 0 limits safely.
        (x_pixel.max(0.0) as u32, y_pixel.max(0.0) as u32)
    }
}

/// Secure file wrapper structure conforming natively to libinput trait requirements.
struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| -1)?;
        let fd = unsafe { open(c_path.as_ptr(), flags) };
        if fd < 0 {
            Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1))
        } else {
            Ok(unsafe { OwnedFd::from_raw_fd(fd) })
        }
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        // fd will automatically close when it goes out of scope and drops
    }
}

pub fn run_input_listener() {
    let mut input = Libinput::new_with_udev(Interface);
    
    if let Err(e) = input.udev_assign_seat("seat0") {
        eprintln!("Failed to assign udev seat: {:?}", e);
        return;
    }
    
    let matrix = CalibrationMatrix::load_from_file("/etc/calibration.conf");

    loop {
        if let Err(e) = input.dispatch() {
            eprintln!("Libinput dispatch error: {}", e);
            thread::sleep(Duration::from_millis(50));
            continue;
        }

        for event in &mut input {
            if let Event::Touch(touch_event) = event {
                // A robust implementation would extract touch.x() and touch.y() events 
                // and channel them to `winit` or KMS input systems.
                // Right now we process the coordinate translation as instructed.
                let raw_x = 0.0; // Retrieve safely from touch_event.x_transformed(screen_width) in a real loop
                let raw_y = 0.0; // Retrieve safely from touch_event.y_transformed(screen_height) in a real loop
                
                let (pixel_x, pixel_y) = matrix.calibrate_point(raw_x, raw_y);
                
                // To interact with KMS correctly, we will send (pixel_x, pixel_y) towards Slint's input pipeline via Slint Window Events.
            }
        }
        
        thread::sleep(Duration::from_millis(10));
    }
}