# VEX IQ Gen 2 Controller Driver

This project allows you to use a VEX IQ Generation 2 Controller as a standard gamepad on your computer. It communicates with the controller via USB Serial and creates a virtual Xbox 360 controller that works with most games and emulators.

## Features

- **USB Connection**: Connects directly via USB-C.
- **Standard Gamepad Emulation**: Emulates a Microsoft Xbox 360 controller for maximum compatibility.
- **Full Mapping**: Supports all buttons and joysticks, including L2/R2 triggers.
- **Low Latency**: Written in Rust for high performance.

## Installation

### Prerequisites

- **Rust**: You need the Rust toolchain installed. [Install Rust](https://rustup.rs/).
- **Linux**: You need `udev` rules to access the device without root (optional but recommended) and `uinput` permissions.

### Setup (Linux)

1.  **Install Dependencies**:
    ```bash
    sudo apt install libudev-dev
    ```

2.  **Clone and Build**:
    ```bash
    git clone https://github.com/yourusername/vex-controller.git
    cd vex-controller
    cargo build --release
    ```

3.  **Setup Permissions**:
    To run without `sudo`, you need to set up udev rules for the VEX Controller and permissions for `uinput`.

    Create a file `/etc/udev/rules.d/99-vex-controller.rules`:
    ```
    SUBSYSTEM=="tty", ATTRS{idVendor}=="2888", ATTRS{idProduct}=="0210", MODE="0666"
    KERNEL=="uinput", MODE="0660", GROUP="input"
    ```
    Then reload rules:
    ```bash
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    ```
    Add your user to the input group:
    ```bash
    sudo usermod -aG input $USER
    ```
    (You may need to log out and back in).

## Usage

Connect your VEX IQ Gen 2 Controller to your PC via USB.

Run the driver in daemon mode to enable the virtual gamepad:

```bash
cargo run --release -- --daemon
```

Or if you installed the binary:

```bash
./target/release/vex-controller --daemon
```

The controller should now appear as "VEX IQ Gen 2 Controller" (spoofing an Xbox 360 controller) in your system settings and games.

### Command Line Options

- `--daemon`: Enable virtual gamepad mode.
- `--port <PORT>`: Manually specify the serial port (e.g., `/dev/ttyACM0`). If not provided, it auto-detects.

## Troubleshooting

- **Permission Denied**: If you get permission errors, try running with `sudo` or check your udev rules.
- **Controller Not Found**: Ensure the controller is turned on and connected via USB. The Brain is not required, just the controller.

