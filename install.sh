#!/bin/bash
set -e

echo "Installing VEX Controller Driver..."

# Build release binary
echo "Building release binary..."
cargo build --release

# Install binary
echo "Installing binary to /usr/local/bin/vex-controller..."
sudo cp target/release/vex-controller /usr/local/bin/
sudo chmod +x /usr/local/bin/vex-controller

# Install udev rules
echo "Installing udev rules..."
sudo cp 99-vex-controller.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger

# Check group membership
if groups $USER | grep &>/dev/null '\binput\b'; then
    echo "User is already in 'input' group."
else
    echo "Adding user to 'input' group..."
    sudo usermod -aG input $USER
    echo "Please log out and log back in for group changes to take effect."
fi

# Install systemd service
echo "Installing systemd service..."
sudo cp vex-controller.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable vex-controller.service
sudo systemctl start vex-controller.service

echo "Installation complete!"
echo "The driver is now running as a service."
echo "You can check its status with: systemctl status vex-controller"

