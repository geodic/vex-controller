use crc::{Crc, CRC_16_XMODEM, CRC_32_ISO_HDLC};
use byteorder::{BigEndian, ByteOrder};

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
    // Add more as needed
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum ResponseStatus {
    AckRx = 0x76,
    NackRx = 0xFF,
    // Add more as needed
}

pub fn calculate_crc16(data: &[u8]) -> u16 {
    CRC16_XMODEM.checksum(data)
}

#[allow(dead_code)]
pub fn calculate_crc32(data: &[u8]) -> u32 {
    CRC32.checksum(data)
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
}

pub struct Protocol;

impl Protocol {
    pub fn new() -> Self {
        Self
    }

    pub fn encode_command(&self, cmd1: u8, cmd2: u8, data: &[u8]) -> Vec<u8> {
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

    pub fn decode_response(&self, buffer: &[u8]) -> Option<Vec<u8>> {
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

    pub fn parse_controller_state(payload: &[u8]) -> Option<ControllerState> {
        // Payload should start with Cmd (0x60)
        // Payload: Cmd(1) + Joy(4) + ... + Buttons(2) + ...
        if payload.len() < 14 || payload[0] != 0x60 {
            return None;
        }

        // JS: e[0]=r[5], e[1]=r[6], e[2]=r[7], e[3]=r[8]
        // payload[0] is Cmd (r[4])
        // payload[1] is r[5] (Left X?)
        // payload[2] is r[6] (Left Y?)
        // payload[3] is r[7] (Right X?)
        // payload[4] is r[8] (Right Y?)
        
        let left_x = payload[1];
        let left_y = payload[2];
        let right_x = payload[3];
        let right_y = payload[4];
        
        // Buttons at payload[8..10] (r[12..14])
        // JS: var o = n.getUint16(12); (Big Endian)
        let buttons = BigEndian::read_u16(&payload[8..10]);
        let extra_buttons = payload[10];
        
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
        })
    }
}
