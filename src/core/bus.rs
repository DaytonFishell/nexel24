use std::ops::{Index, IndexMut};

/// Simple 24-bit address bus with 1MB WorkRAM mapped at 0x000000..=0x0FFFFF
pub struct Bus24 {
    workram: Vec<u8>,
}

impl Bus24 {
    pub const WORKRAM_SIZE: usize = 0x100000; // 1MB

    pub fn new() -> Self {
        Self {
            workram: vec![0; Self::WORKRAM_SIZE],
        }
    }

    /// Read a byte from the 24-bit address space
    pub fn read_u8(&self, addr: u32) -> u8 {
        let a = (addr & 0x00FF_FFFF) as usize;
        if a < Self::WORKRAM_SIZE {
            self.workram[a]
        } else {
            // unmapped reads return 0xff for now
            0xff
        }
    }

    /// Write a byte to the 24-bit address space
    pub fn write_u8(&mut self, addr: u32, value: u8) {
        let a = (addr & 0x00FF_FFFF) as usize;
        if a < Self::WORKRAM_SIZE {
            self.workram[a] = value;
        } else {
            // ignore writes to unmapped regions for now
        }
    }

    /// Read little-endian u16
    pub fn read_u16(&self, addr: u32) -> u16 {
        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    /// Write little-endian u16
    pub fn write_u16(&mut self, addr: u32, v: u16) {
        self.write_u8(addr, (v & 0xFF) as u8);
        self.write_u8(addr.wrapping_add(1), (v >> 8) as u8);
    }
}

impl Default for Bus24 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_read_write_u8() {
        let mut bus = Bus24::new();
        bus.write_u8(0x0000_0000, 0x12);
        assert_eq!(bus.read_u8(0), 0x12);
        bus.write_u8(0x0F_FFFF, 0x34);
        assert_eq!(bus.read_u8(0x0F_FFFF), 0x34);
        // out of range returns 0xff
        assert_eq!(bus.read_u8(0x100000), 0xff);
    }

    #[test]
    fn bus_read_write_u16() {
        let mut bus = Bus24::new();
        bus.write_u16(0x10, 0x1234);
        assert_eq!(bus.read_u16(0x10), 0x1234);
    }
}
