# rKlipperScreen

rKlipperScreen is a lightweight, ultra-high-performance touchscreen GUI that interfaces with [Klipper](https://github.com/Klipper3d/klipper) via [Moonraker](https://github.com/arksine/moonraker). It is a complete rewrite of the original KlipperScreen in Rust using the Slint UI framework.

rKlipperScreen allows you to switch between multiple printers and access them from a single location. Notably, it doesn't need to run on the same host as your printer; you can install it on another device and configure the IP address to connect to the printer.

## Key Features

- **Resource Efficient**: Significantly lower CPU and RAM usage than the legacy Python/GTK3 implementation.
- **Modern Architecture**: Compiled binary with no Python or virtualenv dependencies.
- **Drop-in Configuration**: Fully compatible with existing `KlipperScreen.conf` files.

### Required Hardware

rKlipperScreen should run on any touchscreen that you can connect to a host (Raspberry Pi, PC, Tablet), but not screens that connect directly to the printer MCU board.

A physical touchscreen is not strictly required; you may run a remote desktop/X11 or VNC server to display the UI on client devices. Refer to [Hardware](Hardware.md) for more details.

### Inspiration
rKlipperScreen is inspired by the original Python-based [KlipperScreen](https://github.com/KlipperScreen/KlipperScreen) and [OctoScreen](https://github.com/Z-Bolt/OctoScreen/).
