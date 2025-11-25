# vex-controller
A cross-platform userspace driver enabling the usage of the VEX IQ Gen 2 controller as a HID device.

## Features
- Reads controller state via serial protocol.
- Displays button and joystick status in the terminal.
- **Linux**: Creates a virtual gamepad (HID) using `uinput`.
- **Windows**: Creates a virtual Xbox 360 controller using ViGEmBus.

## Installation

### Linux
1. Clone the repository.
2. Run the install script:
   ```bash
   ./install.sh
   ```
   This will build the project, install the binary to `/usr/local/bin`, set up udev rules, and enable a systemd service for plug-and-play operation.

### Windows
1. Install the [ViGEmBus Driver](https://github.com/ViGEm/ViGEmBus/releases).
2. Run the installation script (PowerShell):
   ```powershell
   .\install.ps1
   ```
   This will build the project and add a shortcut to your Startup folder so the driver runs automatically when you log in.

## Usage

Run the tool from the terminal:

```bash
vex-controller
```

To enable the virtual gamepad:

```bash
vex-controller --daemon
```

## Button Mapping (Virtual Gamepad)

- **Left Stick**: Right Joystick (Inverted Y)
- **Right Stick**: Left Joystick (Inverted Y)
- **L1/L2**: L Up/Down
- **R1/R2**: R Up/Down
- **L3/R3**: Thumbstick Clicks
- **ABXY Diamond**:
  - **Y (North)**: E Up
  - **A (South)**: E Down
  - **X (West)**: F Up
  - **B (East)**: F Down

