# Multiple instances of rKlipperScreen

*Difficulty: Advanced*

This article describes methods of running multiple independent instances of rKlipperScreen (e.g. for multiple connected displays or remote tablet screens via VNC/Xserver).

The instances will run independently of each other.

---

## Performance Notes

Because rKlipperScreen is built in Rust, it is extremely efficient and resource usage is minimal compared to the legacy Python GUI. However, running multiple instances will still consume some system resources:
- Keep track of GPU/CPU limits on low-end hardware like a Raspberry Pi 3A/Zero.
- Use `htop` to monitor system resources.

---

## Option 1: Using the launch script method

Create or edit the launch script:
```sh
nano $HOME/rKlipperScreen/scripts/launch_rKlipperScreen.sh
```

Example script to launch a local screen and a remote display:
```sh
#!/bin/bash
/usr/bin/xinit $KS_XCLIENT &
DISPLAY=192.168.18.147:0 $KS_XCLIENT -c $HOME/.config/KlipperScreen/tablet.cfg &
wait
```

Restart the service to apply changes:
```sh
sudo systemctl restart rKlipperScreen
```

---

## Option 2: Using a separate systemd service

Create a new service unit file:
```sh
sudo nano /etc/systemd/system/rKlipperScreen_tablet.service
```

Example of a service unit:
```ini title="rKlipperScreen_tablet.service"
[Unit]
Description=rKlipperScreen Tablet Instance
After=moonraker.service

[Service]
Type=simple
Restart=always
RestartSec=2
User=pi
WorkingDirectory=/home/pi/rKlipperScreen
Environment="DISPLAY=192.168.18.147:0"
ExecStart=/home/pi/rKlipperScreen/target/release/rklipperscreen -c /home/pi/.config/KlipperScreen/tablet.cfg

[Install]
WantedBy=multi-user.target
```

Reload, enable, and start the new service:
```sh
sudo systemctl daemon-reload
sudo systemctl enable rKlipperScreen_tablet
sudo systemctl start rKlipperScreen_tablet
```

Add the new service to Moonraker's allowed service control configuration in `moonraker.asvc` (if managed through the UI).
```sh
sudo systemctl restart moonraker
```
