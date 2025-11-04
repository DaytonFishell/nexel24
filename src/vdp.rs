//! VDP-T subsystem stub

pub struct Vdp {
    // Simple VRAM placeholder (512KB as per spec)
    vram: Vec<u8>,
    // Registers (64KB region, but we simplify to 256 registers)
    regs: [u8; 256],
}

impl Vdp {
    pub fn new() -> Self {
        Self {
            vram: vec![0; 0x80000], // 512KB
            regs: [0; 256],
        }
    }

    /// Read a byte from VDP register space (offset within 0x100000..0x10FFFF)
    pub fn read_reg(&self, offset: u32) -> u8 {
        self.regs
            .get(offset as usize % 256)
            .copied()
            .unwrap_or(0xFF)
    }

    /// Write a byte to VDP register space
    pub fn write_reg(&mut self, offset: u32, value: u8) {
        if let Some(slot) = self.regs.get_mut(offset as usize % 256) {
            *slot = value;
        }
    }

    /// Read a byte from VRAM (offset within 0x200000..0x27FFFF)
    pub fn read_vram(&self, offset: u32) -> u8 {
        self.vram
            .get(offset as usize % self.vram.len())
            .copied()
            .unwrap_or(0xFF)
    }

    /// Write a byte to VRAM
    pub fn write_vram(&mut self, offset: u32, value: u8) {
        // compute length/index first so the immutable borrow from `.len()` ends
        // before we take a mutable borrow with `get_mut`.
        let len = self.vram.len();
        let idx = offset as usize % len;
        if let Some(cell) = self.vram.get_mut(idx) {
            *cell = value;
        }
    }
}
