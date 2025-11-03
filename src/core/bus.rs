/// 24-bit address bus with full memory map support
/// 
/// Memory Map (per Nexel-24 specification):
/// - 0x000000..0x00FFFF: WorkRAM (64KB) - Primary stack/heap
/// - 0x010000..0x03FFFF: ExpandedRAM (192KB)
/// - 0x100000..0x10FFFF: I/O (64KB) - Memory-mapped coprocessors
/// - 0x200000..0x27FFFF: VRAM (512KB)
/// - 0x280000..0x28FFFF: CRAM (64KB)
/// - 0x400000..0x9FFFFF: CartROM (6MB max)
/// - 0xA00000..0xA3FFFF: CartSave (256KB)
/// - 0xFF0000..0xFFFFFF: BIOS (64KB)
pub struct Bus24 {
    workram: Vec<u8>,       // 0x000000..0x00FFFF (64KB)
    expanded_ram: Vec<u8>,  // 0x010000..0x03FFFF (192KB)
    io: Vec<u8>,            // 0x100000..0x10FFFF (64KB) - I/O registers
    vram: Vec<u8>,          // 0x200000..0x27FFFF (512KB)
    cram: Vec<u8>,          // 0x280000..0x28FFFF (64KB)
    cart_rom: Vec<u8>,      // 0x400000..0x9FFFFF (6MB)
    cart_save: Vec<u8>,     // 0xA00000..0xA3FFFF (256KB)
    bios: Vec<u8>,          // 0xFF0000..0xFFFFFF (64KB)
}

impl Bus24 {
    // Memory region sizes
    pub const WORKRAM_SIZE: usize = 0x010000;      // 64KB
    pub const EXPANDED_RAM_SIZE: usize = 0x030000; // 192KB
    pub const IO_SIZE: usize = 0x010000;           // 64KB
    pub const VRAM_SIZE: usize = 0x080000;         // 512KB
    pub const CRAM_SIZE: usize = 0x010000;         // 64KB
    pub const CART_ROM_SIZE: usize = 0x600000;     // 6MB
    pub const CART_SAVE_SIZE: usize = 0x040000;    // 256KB
    pub const BIOS_SIZE: usize = 0x010000;         // 64KB

    // Memory region base addresses
    pub const WORKRAM_BASE: u32 = 0x000000;
    pub const EXPANDED_RAM_BASE: u32 = 0x010000;
    pub const IO_BASE: u32 = 0x100000;
    pub const VRAM_BASE: u32 = 0x200000;
    pub const CRAM_BASE: u32 = 0x280000;
    pub const CART_ROM_BASE: u32 = 0x400000;
    pub const CART_SAVE_BASE: u32 = 0xA00000;
    pub const BIOS_BASE: u32 = 0xFF0000;

    pub fn new() -> Self {
        Self {
            workram: vec![0; Self::WORKRAM_SIZE],
            expanded_ram: vec![0; Self::EXPANDED_RAM_SIZE],
            io: vec![0; Self::IO_SIZE],
            vram: vec![0; Self::VRAM_SIZE],
            cram: vec![0; Self::CRAM_SIZE],
            cart_rom: vec![0; Self::CART_ROM_SIZE],
            cart_save: vec![0; Self::CART_SAVE_SIZE],
            bios: vec![0; Self::BIOS_SIZE],
        }
    }

    /// Load cartridge ROM data
    pub fn load_cart_rom(&mut self, data: &[u8]) {
        let len = data.len().min(Self::CART_ROM_SIZE);
        self.cart_rom[..len].copy_from_slice(&data[..len]);
    }

    /// Load BIOS data
    pub fn load_bios(&mut self, data: &[u8]) {
        let len = data.len().min(Self::BIOS_SIZE);
        self.bios[..len].copy_from_slice(&data[..len]);
    }

    /// Read a byte from the 24-bit address space
    pub fn read_u8(&self, addr: u32) -> u8 {
        let addr = addr & 0x00FF_FFFF; // Mask to 24-bit
        
        match addr {
            // WorkRAM: 0x000000..0x00FFFF
            a if a < Self::EXPANDED_RAM_BASE => {
                self.workram[a as usize]
            }
            // ExpandedRAM: 0x010000..0x03FFFF
            a if a >= Self::EXPANDED_RAM_BASE && a < 0x040000 => {
                let offset = (a - Self::EXPANDED_RAM_BASE) as usize;
                self.expanded_ram[offset]
            }
            // I/O: 0x100000..0x10FFFF
            a if a >= Self::IO_BASE && a < Self::IO_BASE + Self::IO_SIZE as u32 => {
                let offset = (a - Self::IO_BASE) as usize;
                self.io[offset]
            }
            // VRAM: 0x200000..0x27FFFF
            a if a >= Self::VRAM_BASE && a < Self::VRAM_BASE + Self::VRAM_SIZE as u32 => {
                let offset = (a - Self::VRAM_BASE) as usize;
                self.vram[offset]
            }
            // CRAM: 0x280000..0x28FFFF
            a if a >= Self::CRAM_BASE && a < Self::CRAM_BASE + Self::CRAM_SIZE as u32 => {
                let offset = (a - Self::CRAM_BASE) as usize;
                self.cram[offset]
            }
            // CartROM: 0x400000..0x9FFFFF
            a if a >= Self::CART_ROM_BASE && a < Self::CART_ROM_BASE + Self::CART_ROM_SIZE as u32 => {
                let offset = (a - Self::CART_ROM_BASE) as usize;
                self.cart_rom[offset]
            }
            // CartSave: 0xA00000..0xA3FFFF
            a if a >= Self::CART_SAVE_BASE && a < Self::CART_SAVE_BASE + Self::CART_SAVE_SIZE as u32 => {
                let offset = (a - Self::CART_SAVE_BASE) as usize;
                self.cart_save[offset]
            }
            // BIOS: 0xFF0000..0xFFFFFF
            a if a >= Self::BIOS_BASE => {
                let offset = (a - Self::BIOS_BASE) as usize;
                if offset < Self::BIOS_SIZE {
                    self.bios[offset]
                } else {
                    0xFF // Unmapped high region
                }
            }
            // Unmapped regions return 0xFF
            _ => 0xFF,
        }
    }

    /// Write a byte to the 24-bit address space
    pub fn write_u8(&mut self, addr: u32, value: u8) {
        let addr = addr & 0x00FF_FFFF; // Mask to 24-bit
        
        match addr {
            // WorkRAM: 0x000000..0x00FFFF
            a if a < Self::EXPANDED_RAM_BASE => {
                self.workram[a as usize] = value;
            }
            // ExpandedRAM: 0x010000..0x03FFFF
            a if a >= Self::EXPANDED_RAM_BASE && a < 0x040000 => {
                let offset = (a - Self::EXPANDED_RAM_BASE) as usize;
                self.expanded_ram[offset] = value;
            }
            // I/O: 0x100000..0x10FFFF
            a if a >= Self::IO_BASE && a < Self::IO_BASE + Self::IO_SIZE as u32 => {
                let offset = (a - Self::IO_BASE) as usize;
                self.io[offset] = value;
            }
            // VRAM: 0x200000..0x27FFFF
            a if a >= Self::VRAM_BASE && a < Self::VRAM_BASE + Self::VRAM_SIZE as u32 => {
                let offset = (a - Self::VRAM_BASE) as usize;
                self.vram[offset] = value;
            }
            // CRAM: 0x280000..0x28FFFF
            a if a >= Self::CRAM_BASE && a < Self::CRAM_BASE + Self::CRAM_SIZE as u32 => {
                let offset = (a - Self::CRAM_BASE) as usize;
                self.cram[offset] = value;
            }
            // CartROM: 0x400000..0x9FFFFF (read-only, writes ignored)
            a if a >= Self::CART_ROM_BASE && a < Self::CART_ROM_BASE + Self::CART_ROM_SIZE as u32 => {
                // ROM is read-only, ignore writes
            }
            // CartSave: 0xA00000..0xA3FFFF
            a if a >= Self::CART_SAVE_BASE && a < Self::CART_SAVE_BASE + Self::CART_SAVE_SIZE as u32 => {
                let offset = (a - Self::CART_SAVE_BASE) as usize;
                self.cart_save[offset] = value;
            }
            // BIOS: 0xFF0000..0xFFFFFF (read-only, writes ignored)
            a if a >= Self::BIOS_BASE => {
                // BIOS is read-only, ignore writes
            }
            // Unmapped regions, ignore writes
            _ => {}
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

    /// Read little-endian 24-bit value (returned as u32)
    pub fn read_u24(&self, addr: u32) -> u32 {
        let lo = self.read_u8(addr) as u32;
        let mid = self.read_u8(addr.wrapping_add(1)) as u32;
        let hi = self.read_u8(addr.wrapping_add(2)) as u32;
        lo | (mid << 8) | (hi << 16)
    }

    /// Write little-endian 24-bit value (from u32, upper 8 bits ignored)
    pub fn write_u24(&mut self, addr: u32, v: u32) {
        self.write_u8(addr, (v & 0xFF) as u8);
        self.write_u8(addr.wrapping_add(1), ((v >> 8) & 0xFF) as u8);
        self.write_u8(addr.wrapping_add(2), ((v >> 16) & 0xFF) as u8);
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
    fn bus_read_write_workram() {
        let mut bus = Bus24::new();
        // Test WorkRAM region (0x000000..0x00FFFF)
        bus.write_u8(0x000000, 0x12);
        assert_eq!(bus.read_u8(0x000000), 0x12);
        bus.write_u8(0x00FFFF, 0x34);
        assert_eq!(bus.read_u8(0x00FFFF), 0x34);
    }

    #[test]
    fn bus_read_write_expanded_ram() {
        let mut bus = Bus24::new();
        // Test ExpandedRAM region (0x010000..0x03FFFF)
        bus.write_u8(0x010000, 0xAB);
        assert_eq!(bus.read_u8(0x010000), 0xAB);
        bus.write_u8(0x03FFFF, 0xCD);
        assert_eq!(bus.read_u8(0x03FFFF), 0xCD);
    }

    #[test]
    fn bus_read_write_io() {
        let mut bus = Bus24::new();
        // Test I/O region (0x100000..0x10FFFF)
        bus.write_u8(0x100000, 0x55);
        assert_eq!(bus.read_u8(0x100000), 0x55);
        bus.write_u8(0x10FFFF, 0xAA);
        assert_eq!(bus.read_u8(0x10FFFF), 0xAA);
    }

    #[test]
    fn bus_read_write_vram() {
        let mut bus = Bus24::new();
        // Test VRAM region (0x200000..0x27FFFF)
        bus.write_u8(0x200000, 0x11);
        assert_eq!(bus.read_u8(0x200000), 0x11);
        bus.write_u8(0x27FFFF, 0x22);
        assert_eq!(bus.read_u8(0x27FFFF), 0x22);
    }

    #[test]
    fn bus_read_write_cram() {
        let mut bus = Bus24::new();
        // Test CRAM region (0x280000..0x28FFFF)
        bus.write_u8(0x280000, 0x33);
        assert_eq!(bus.read_u8(0x280000), 0x33);
        bus.write_u8(0x28FFFF, 0x44);
        assert_eq!(bus.read_u8(0x28FFFF), 0x44);
    }

    #[test]
    fn bus_read_cart_rom() {
        let mut bus = Bus24::new();
        // Load some test ROM data
        let rom_data = vec![0x12, 0x34, 0x56, 0x78];
        bus.load_cart_rom(&rom_data);
        
        // Test CartROM region (0x400000..0x9FFFFF)
        assert_eq!(bus.read_u8(0x400000), 0x12);
        assert_eq!(bus.read_u8(0x400001), 0x34);
        assert_eq!(bus.read_u8(0x400002), 0x56);
        assert_eq!(bus.read_u8(0x400003), 0x78);
    }

    #[test]
    fn bus_rom_is_readonly() {
        let mut bus = Bus24::new();
        let rom_data = vec![0xFF; 4];
        bus.load_cart_rom(&rom_data);
        
        // Try to write to ROM (should be ignored)
        bus.write_u8(0x400000, 0x00);
        assert_eq!(bus.read_u8(0x400000), 0xFF); // Should still be 0xFF
    }

    #[test]
    fn bus_read_write_cart_save() {
        let mut bus = Bus24::new();
        // Test CartSave region (0xA00000..0xA3FFFF)
        bus.write_u8(0xA00000, 0x99);
        assert_eq!(bus.read_u8(0xA00000), 0x99);
        bus.write_u8(0xA3FFFF, 0x88);
        assert_eq!(bus.read_u8(0xA3FFFF), 0x88);
    }

    #[test]
    fn bus_read_bios() {
        let mut bus = Bus24::new();
        let bios_data = vec![0xAB, 0xCD, 0xEF, 0x01];
        bus.load_bios(&bios_data);
        
        // Test BIOS region (0xFF0000..0xFFFFFF)
        assert_eq!(bus.read_u8(0xFF0000), 0xAB);
        assert_eq!(bus.read_u8(0xFF0001), 0xCD);
        assert_eq!(bus.read_u8(0xFF0002), 0xEF);
        assert_eq!(bus.read_u8(0xFF0003), 0x01);
    }

    #[test]
    fn bus_bios_is_readonly() {
        let mut bus = Bus24::new();
        let bios_data = vec![0xFF; 4];
        bus.load_bios(&bios_data);
        
        // Try to write to BIOS (should be ignored)
        bus.write_u8(0xFF0000, 0x00);
        assert_eq!(bus.read_u8(0xFF0000), 0xFF); // Should still be 0xFF
    }

    #[test]
    fn bus_unmapped_reads_return_ff() {
        let bus = Bus24::new();
        // Test unmapped regions return 0xFF
        assert_eq!(bus.read_u8(0x040000), 0xFF); // Between ExpandedRAM and I/O
        assert_eq!(bus.read_u8(0x290000), 0xFF); // After CRAM
        assert_eq!(bus.read_u8(0xA40000), 0xFF); // After CartSave
    }

    #[test]
    fn bus_read_write_u16() {
        let mut bus = Bus24::new();
        bus.write_u16(0x10, 0x1234);
        assert_eq!(bus.read_u16(0x10), 0x1234);
        
        // Test little-endian byte order
        bus.write_u8(0x20, 0x78);
        bus.write_u8(0x21, 0x56);
        assert_eq!(bus.read_u16(0x20), 0x5678);
    }

    #[test]
    fn bus_read_write_u24() {
        let mut bus = Bus24::new();
        bus.write_u24(0x100, 0x123456);
        assert_eq!(bus.read_u24(0x100), 0x123456);
        
        // Test little-endian byte order
        bus.write_u8(0x200, 0x34);
        bus.write_u8(0x201, 0x12);
        bus.write_u8(0x202, 0xAB);
        assert_eq!(bus.read_u24(0x200), 0xAB1234);
    }

    #[test]
    fn bus_address_masking() {
        let mut bus = Bus24::new();
        // Addresses should be masked to 24-bit
        bus.write_u8(0x01000000, 0x42); // Should wrap to 0x000000
        assert_eq!(bus.read_u8(0x000000), 0x42);
    }
}
