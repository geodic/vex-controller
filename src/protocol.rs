use crate::transport::Transport;
use anyhow::{Result, bail};
use byteorder::{BigEndian, ByteOrder};
use crc::{Crc, CRC_16_XMODEM, CRC_32_ISO_HDLC};
use std::time::{Duration, Instant};
use tracing::debug;

pub const HEADERS: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];
pub const HEADERR: [u8; 2] = [0xAA, 0x55];

pub const CRC16_XMODEM: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);
#[allow(dead_code)]
pub const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Command {
    SysStatus = 0x20,
    FileInit = 0x11,
    FactoryPing = 0xF4,
    ControllerCdc = 0x58,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum ControllerSubCommand {
    GetState = 0x60,
    SetPairId = 0x61,
    GetPairId = 0x62,
    GetTestData = 0x63,
    TestCmd = 0x64,
    AbortJsCal = 0x65,
    StartJsCal = 0x66,
    GetVersions = 0x67,
    DevState = 0x68,
}

pub fn calculate_crc16(data: &[u8]) -> u16 {
    CRC16_XMODEM.checksum(data)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ControllerState {
    pub left_x: u8,
    pub left_y: u8,
    pub right_x: u8,
    pub right_y: u8,
    pub l_up: bool,
    pub l_down: bool,
    pub r_up: bool,
    pub r_down: bool,
    pub e_up: bool,
    pub e_down: bool,
    pub f_up: bool,
    pub f_down: bool,
    pub l3: bool,
    pub r3: bool,
    pub battery: u8,
    pub cal_active: bool,
    pub cal_left: bool,
    pub cal_right: bool,
}

struct Protocol;

impl Protocol {
    fn encode_command(cmd1: u8, cmd2: u8, data: &[u8]) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&HEADERS);
        packet.push(cmd1);
        packet.push(cmd2);

        if data.len() < 128 {
            packet.push(data.len() as u8);
            packet.extend_from_slice(data);
        } else {
            let len = data.len();
            packet.push(((len >> 8) as u8) | 0x80);
            packet.push((len & 0xFF) as u8);
            packet.extend_from_slice(data);
        }

        let crc = calculate_crc16(&packet);
        packet.push((crc >> 8) as u8);
        packet.push((crc & 0xFF) as u8);

        packet
    }

    fn decode_response(buffer: &[u8]) -> Option<Vec<u8>> {
        // Basic validation
        if buffer.len() < 5 {
            return None;
        }

        // Check header
        if buffer[0..2] != HEADERR {
            return None;
        }

        // Extract data
        // Length parsing (at index 3)
        let (len, header_size) = if (buffer[3] & 0x80) != 0 {
            if buffer.len() < 5 { return None; }
            let len = ((buffer[3] & 0x7F) as usize) << 8 | (buffer[4] as usize);
            (len, 5)
        } else {
            (buffer[3] as usize, 4)
        };

        // Check if we have enough data
        // len includes Payload + CRC
        let packet_len = header_size + len;
        if buffer.len() < packet_len {
            return None;
        }
        
        let packet = &buffer[..packet_len];

        // Check CRC
        let received_crc = BigEndian::read_u16(&packet[packet_len - 2..]);
        let calculated_crc = calculate_crc16(&packet[..packet_len - 2]);

        if received_crc != calculated_crc {
            return None;
        }

        // Return payload, excluding CRC
        Some(packet[header_size..packet_len - 2].to_vec())
    }

    fn parse_controller_state(payload: &[u8]) -> Option<ControllerState> {
        if payload.len() < 14 || payload[0] != 0x60 {
            return None;
        }

        let left_x = payload[1];
        let left_y = payload[2];
        let right_x = payload[3];
        let right_y = payload[4];
        
        let buttons = BigEndian::read_u16(&payload[8..10]);
        let extra_buttons = payload[10];
        
        let status = payload[8];
        
        Some(ControllerState {
            left_x,
            left_y,
            right_x,
            right_y,
            l_up: (buttons >> 5) & 1 != 0,
            l_down: (buttons >> 4) & 1 != 0,
            r_up: (buttons >> 7) & 1 != 0,
            r_down: (buttons >> 6) & 1 != 0,
            e_up: (buttons >> 3) & 1 != 0,
            e_down: (buttons >> 1) & 1 != 0,
            f_up: (buttons >> 2) & 1 != 0,
            f_down: (buttons >> 0) & 1 != 0,
            l3: (extra_buttons & 0x01) != 0,
            r3: (extra_buttons & 0x02) != 0,
            battery: payload[11], 
            cal_active: (status >> 4) & 1 != 0,
            cal_left: (status >> 5) & 1 != 0,
            cal_right: (status >> 6) & 1 != 0,
        })
    }
}

pub struct VexController {
    transport: Box<dyn Transport>,
}

impl VexController {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }

    pub fn send_command(&mut self, cmd1: u8, cmd2: u8, data: &[u8]) -> Result<Vec<u8>> {
        let command = Protocol::encode_command(cmd1, cmd2, data);
        
        self.transport.clear_buffer()?;
        self.transport.send_bytes(&command)?;

        let mut buffer = vec![0u8; 1024];
        let mut packet_buffer = Vec::new();
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(2) {
            let n = self.transport.receive_bytes(&mut buffer)?;
            if n > 0 {
                packet_buffer.extend_from_slice(&buffer[..n]);
                
                // Framing loop
                while packet_buffer.len() >= 2 {
                    if let Some(start_idx) = packet_buffer.windows(2).position(|w| w == HEADERR) {
                        if start_idx > 0 {
                            packet_buffer.drain(0..start_idx);
                        }
                        
                        if packet_buffer.len() < 5 {
                            break;
                        }

                        let (len, header_size) = if (packet_buffer[3] & 0x80) != 0 {
                            let len = ((packet_buffer[3] & 0x7F) as usize) << 8 | (packet_buffer[4] as usize);
                            (len, 5)
                        } else {
                            (packet_buffer[3] as usize, 4)
                        };
                        
                        let packet_len = header_size + len;
                        if packet_buffer.len() >= packet_len {
                            let packet = &packet_buffer[..packet_len];
                            if let Some(payload) = Protocol::decode_response(packet) {
                                debug!("Raw response: {:02X?}", payload);
                                return Ok(payload);
                            } else {
                                // CRC failed or invalid, remove header and try again
                                packet_buffer.drain(0..2);
                            }
                        } else {
                            break;
                        }
                    } else {
                        packet_buffer.clear();
                        break;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        
        bail!("Timeout waiting for response");
    }

    pub fn get_state(&mut self) -> Result<ControllerState> {
        let payload = self.send_command(Command::ControllerCdc as u8, ControllerSubCommand::GetState as u8, &[])?;
        Protocol::parse_controller_state(&payload).ok_or_else(|| anyhow::anyhow!("Failed to parse state"))
    }

    pub fn get_versions(&mut self) -> Result<String> {
        let payload = self.send_command(Command::ControllerCdc as u8, ControllerSubCommand::GetVersions as u8, &[])?;
        if payload.len() > 1 {
            Ok(String::from_utf8_lossy(&payload[1..]).to_string())
        } else {
            bail!("Invalid version payload")
        }
    }

    pub fn get_pair_id(&mut self) -> Result<u8> {
        let payload = self.send_command(Command::ControllerCdc as u8, ControllerSubCommand::GetPairId as u8, &[])?;
        if payload.len() > 1 {
            Ok(payload[1])
        } else {
            bail!("Invalid pair ID payload")
        }
    }

    pub fn start_calibration(&mut self) -> Result<()> {
        self.send_command(Command::ControllerCdc as u8, ControllerSubCommand::StartJsCal as u8, &[])?;
        Ok(())
    }

    pub fn abort_calibration(&mut self) -> Result<()> {
        self.send_command(Command::ControllerCdc as u8, ControllerSubCommand::AbortJsCal as u8, &[])?;
        Ok(())
    }
}
