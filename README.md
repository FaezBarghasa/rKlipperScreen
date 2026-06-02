# rKlipperScreen

rKlipperScreen is a lightweight, ultra-high-performance touchscreen GUI for [Klipper](https://github.com/Klipper3d/klipper) 3D printers, interfacing via [Moonraker](https://github.com/arksine/moonraker). 

It is a modern port and drop-in replacement for the original KlipperScreen, rewritten in **Rust** using the **Slint** UI framework.

## Why rKlipperScreen?

- **Ultra-Lightweight & Fast**: Eliminates the overhead of a Python virtual environment and GTK3/GObject runtime, leading to minimal memory usage and extremely fast startup times.
- **Modern UI**: Rendered using the Slint UI framework, providing a fluid, GPU-accelerated interface.
- **Drop-in Compatibility**: Reads and parses existing `KlipperScreen.conf` files, custom menu structures, preheat profiles, and handles Moonraker API communication.

---

## Build & Run

### Prerequisites
Make sure you have Rust and Cargo installed. If not, install via [rustup](https://rustup.rs/):
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
Run the compiled binary:
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
Translations are handled via the Slint translation features and integrated dictionaries.

## About the Project
rKlipperScreen is an open-source project building on the incredible design and concepts of KlipperScreen.
