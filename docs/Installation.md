# Installation

## First Steps

1. **Install the Screen**: Follow the manufacturer’s instructions for installing your screen. Some screens may require additional software, while others might not.
2. **Test the Screen**: Ensure your hardware is functioning correctly by testing it with RaspberryOS, Ubuntu, or your preferred distribution.
3. **Proceed to Install rKlipperScreen**: Once you’ve confirmed that the screen is working, you can proceed with installing rKlipperScreen.

---

## Manual Install & Compile

To install rKlipperScreen, you will compile the native binary from source:

1. **Install System Dependencies**:
   Before compiling, ensure you have the required native development headers for hardware input and rendering:
   ```sh
   sudo apt-get update
   sudo apt-get install -y libudev-dev libinput-dev
   ```

2. **Install Rust & Cargo**:
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

3. **Clone the Repository**:
   ```sh
   cd ~/
   git clone https://github.com/FaezBarghasa/rKlipperScreen.git
   cd rKlipperScreen
   ```

3. **Build the Release Binary**:
   ```sh
   cargo build --release
   ```

---

## Systemd Service Configuration

Create a systemd unit file to manage the rKlipperScreen service:

```sh
sudo nano /etc/systemd/system/rKlipperScreen.service
```

Add the following configuration:

```ini
[Unit]
Description=rKlipperScreen Slint GUI
After=moonraker.service

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi/rKlipperScreen
ExecStart=/home/pi/rKlipperScreen/target/release/rklipperscreen
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
```

Enable and start the service:
```sh
sudo systemctl daemon-reload
sudo systemctl enable rKlipperScreen
sudo systemctl start rKlipperScreen
```

---

## Moonraker Configuration

1. Ensure that the IP of the device is a trusted client in `moonraker.conf`:
    ```ini
    [authorization]
    trusted_clients:
      127.0.0.1
    ```
   Alternatively, add the [Moonraker API key](https://moonraker.readthedocs.io/en/latest/installation/#retrieving-the-api-key) to `KlipperScreen.conf`.

2. To use the update manager feature of Moonraker for rKlipperScreen, add the following block to `moonraker.conf`:
    ```ini
    [update_manager rKlipperScreen]
    type: git_repo
    path: ~/rKlipperScreen
    origin: https://github.com/FaezBarghasa/rKlipperScreen.git
    managed_services: rKlipperScreen
    ```

---

## Printer Configuration

Add the following basic configurations to your `printer.cfg` file for correct functionality:
```ini
[virtual_sdcard]
path: ~/printer_data/gcodes
[display_status]
[pause_resume]
```