# rKlipperScreen

rKlipperScreen is a lightweight, ultra-high-performance touchscreen GUI for [Klipper](https://github.com/Klipper3d/klipper) 3D printers, interfacing via [Moonraker](https://github.com/arksine/moonraker).

It is a modern port and drop-in replacement for the original KlipperScreen, rewritten in **Rust** using the **Slint** UI framework.

## Features

*   **Ultra-Lightweight & Fast**: Experience minimal memory usage and extremely fast startup times by eliminating the overhead of a Python virtual environment and GTK3/GObject runtime.
*   **Modern & Responsive UI**: Enjoy a fluid, GPU-accelerated interface powered by the Slint UI framework.
*   **Seamless KlipperScreen Compatibility**: Directly reads and parses existing `KlipperScreen.conf` files, custom menu structures, and preheat profiles.
*   **Multi-Backend Rendering**: Automatically detects the graphical stack to run on **Wayland (via Cage/Sway)**, **X11**, or **Bare-Metal Direct KMS (DRM)** framebuffers seamlessly.
*   **Native Async Touch**: Intercepts and dynamically translates raw touchscreen data via native asynchronous `evdev` integration, skipping overhead from display managers.
*   **Robust Moonraker Integration**: Handles all Moonraker API communication efficiently.
*   **Cross-Platform Potential**: Built with Rust and Slint, offering potential for broader platform support.

## Why rKlipperScreen?

rKlipperScreen offers a superior alternative for Klipper users seeking enhanced performance, a modern user experience, and a more resource-efficient solution. By leveraging Rust and Slint, it provides a snappier, more reliable interface for controlling your 3D printer.

---

## Build & Run

### Prerequisites
Ensure you have Rust and Cargo installed. If not, install via [rustup](https://rustup.rs/):
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Building the Project
Clone the repository and compile the release binary:
```sh
git clone https://github.com/FaezBarghasa/rKlipperScreen.git
cd rKlipperScreen
cargo build --release
```

### Running rKlipperScreen
Execute the compiled binary from the project root:
```sh
./target/release/rklipperscreen [options]
```

#### Command Line Options
```
rKlipperScreen Slint Port

Usage: rklipperscreen [OPTIONS]

Options:
  -c, --config <CONFIG>    Path to config file [default: ~/KlipperScreen.conf]
  -l, --logfile <LOGFILE>  Path to log file [default: /tmp/KlipperScreen.log]
  -a, --address <ADDRESS>  Moonraker host address [default: 127.0.0.1]
  -p, --port <PORT>        Moonraker port [default: 7125]
  -h, --help               Print help
  -V, --version            Print version
```

---

## Translations
Translations are handled via the Slint translation features and integrated dictionaries, making it easy to localize the interface.

## Contributing
We welcome contributions! If you're interested in improving rKlipperScreen, please check out our [contribution guidelines](CONTRIBUTING.md) (if available) or open an issue/pull request.

## About the Project
rKlipperScreen is an open-source project that builds upon the incredible design and concepts of KlipperScreen, aiming to provide a modern, high-performance alternative for the Klipper community.
