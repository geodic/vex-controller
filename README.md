# VEX Controller BLE Protocol

This project implements the VEX Robotics Bluetooth Low Energy (BLE) protocol for communicating with VEX IQ (and likely V5/EXP) Brains.

## Protocol Details

### BLE Service & Characteristics

**Service UUID**: `08590f7e-db05-467e-8757-72f6faeb13d5`

| Characteristic | UUID | Type | Description |
|---|---|---|---|
| **RX Admin** | `08590f7e-db05-467e-8757-72f6faeb13f5` | Write | Send System Commands to Brain |
| **TX Admin** | `08590f7e-db05-467e-8757-72f6faeb1306` | Notify | Receive System Responses from Brain |
| **RX User** | `08590f7e-db05-467e-8757-72f6faeb1326` | Write | Send User Data (Controller State) |
| **TX User** | `08590f7e-db05-467e-8757-72f6faeb1316` | Notify | Receive User Data |
| **RX Lock** | `08590f7e-db05-467e-8757-72f6faeb13e5` | Write | Unlock/Lock Brain? |

### Packet Structure

The protocol uses a custom CDC-like packet structure over BLE.

**Command Packet (Host -> Brain):**
`[Header 4B] [Cmd1 1B] [Cmd2 1B] [Length 1-2B] [Payload N] [CRC 2B]`

- **Header**: `C9 36 B8 47`
- **Length**:
  - If `Length < 128`: 1 byte
  - If `Length >= 128`: 2 bytes (High bit set on first byte)
- **CRC**: CRC16-XMODEM (Poly `0x1021`) of the entire packet (excluding CRC itself).

**Response Packet (Brain -> Host):**
`[Header 2B] [Cmd 1B] [Length 1-2B] [Payload N] [CRC 2B]`

- **Header**: `AA 55`

### Commands

- `SYS_STATUS` (`0x20`): Request system status.
- `FILE_INIT` (`0x11`): Initialize file transfer.
- `FACTORY_PING` (`0xF4`): Ping.

### Implementation

The Rust implementation uses `btleplug` for BLE communication and `tokio` for async execution.
It connects to the first VEX device found, subscribes to the Admin and User notification channels, and sends a ping.
It then listens for incoming data and attempts to decode it.

