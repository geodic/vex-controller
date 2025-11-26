#![allow(dead_code)]

pub struct Crc16 {
    table: [u16; 256],
}

impl Crc16 {
    pub fn new() -> Self {
        let mut table = [0u16; 256];
        for i in 0..256 {
            let mut r = (i as u16) << 8;
            for _ in 0..8 {
                if (r & 0x8000) != 0 {
                    r = (r << 1) ^ 0x1021;
                } else {
                    r <<= 1;
                }
            }
            table[i] = r;
        }
        Self { table }
    }

    pub fn compute(&self, data: &[u8], initial: u16) -> u16 {
        let mut crc = initial;
        for &byte in data {
            let index = ((crc >> 8) ^ (byte as u16)) as u8;
            crc = (crc << 8) ^ self.table[index as usize];
        }
        crc
    }
}
