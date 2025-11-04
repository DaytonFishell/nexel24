//! Main Nexel-24 emulator integration
//!
//! This module provides the main emulator struct that coordinates the CPU,
//! memory bus, and coprocessors.

use crate::apu::Apu;
use crate::core::Bus24;
use crate::cpu::Cpu;
use crate::vdp::Vdp;
use crate::vlu::Vlu;
use crate::vm::BaseplateVm;

/// Main Nexel-24 emulator state
pub struct Nexel24 {
    pub cpu: Cpu,
    pub bus: Bus24,
    pub vdp: Vdp,
    pub vlu: Vlu,
    pub apu: Apu,
    pub vm: Option<BaseplateVm>,

    // Frame timing
    pub frame_count: u64,
    pub target_cycles_per_frame: u64,
}

impl Nexel24 {
    /// CPU clock: 18.432 MHz
    pub const CPU_CLOCK_HZ: u64 = 18_432_000;

    /// Target framerate: 60 Hz (NTSC)
    pub const TARGET_FPS: u64 = 60;

    /// Cycles per frame at 60 FPS
    pub const CYCLES_PER_FRAME: u64 = Self::CPU_CLOCK_HZ / Self::TARGET_FPS; // 307,200 cycles

    /// Create a new emulator instance
    pub fn new() -> Self {
        let mut bus = Bus24::new();
        bus.enable_vdp_routing(); // Enable VDP routing through emulator

        Self {
            cpu: Cpu::new(),
            bus,
            vdp: Vdp::new(),
            vlu: Vlu::new(),
            apu: Apu::new(),
            vm: None,
            frame_count: 0,
            target_cycles_per_frame: Self::CYCLES_PER_FRAME,
        }
    }

    /// Reset the entire system
    pub fn reset(&mut self) {
        self.cpu.reset(&self.bus);
        self.frame_count = 0;
    }

    /// Load a BIOS ROM
    pub fn load_bios(&mut self, data: &[u8]) {
        self.bus.load_bios(data);
    }

    /// Load a cartridge ROM
    pub fn load_cartridge(&mut self, data: &[u8]) {
        self.bus.load_cart_rom(data);
    }

    /// Execute a single CPU instruction with VDP routing
    pub fn step(&mut self) {
        self.cpu.step(&mut self.bus);

        // VDP runs in parallel, advance it by the same number of cycles
        // TODO: Properly track cycles per instruction
        self.vdp.step(1);
    }

    /// Execute instructions for one frame (approximately 307,200 cycles at 60 FPS)
    pub fn step_frame(&mut self) {
        let start_cycles = self.cpu.cycles;
        let target_cycles = start_cycles + self.target_cycles_per_frame;

        while self.cpu.cycles < target_cycles && !self.cpu.halted {
            let cycles_before = self.cpu.cycles;
            self.cpu.step(&mut self.bus);
            let cycles_elapsed = self.cpu.cycles - cycles_before;

            // Advance VDP by the same number of cycles
            let vblank_triggered = self.vdp.step(cycles_elapsed);

            // TODO: Handle VBLANK interrupt
            if vblank_triggered && self.vdp.in_vblank() {
                // Trigger VBLANK interrupt to CPU if enabled
            }
        }

        self.frame_count += 1;
    }

    /// Read from memory with VDP routing
    pub fn read_memory(&self, addr: u32) -> u8 {
        let addr = addr & 0x00FFFFFF;

        // Route VDP regions
        match addr {
            // VDP-T registers: 0x100000..0x103FFF
            a if a >= Bus24::VDP_IO_BASE && a < Bus24::VDP_IO_BASE + 0x4000 => {
                let offset = a - Bus24::VDP_IO_BASE;
                // VDP registers are 16-bit, read as bytes
                if offset & 1 == 0 {
                    (self.vdp.read_reg(offset) & 0xFF) as u8
                } else {
                    (self.vdp.read_reg(offset - 1) >> 8) as u8
                }
            }
            // VRAM: 0x200000..0x27FFFF
            a if a >= Bus24::VRAM_BASE && a < Bus24::VRAM_BASE + 0x80000 => {
                let offset = a - Bus24::VRAM_BASE;
                self.vdp.read_vram(offset)
            }
            // CRAM: 0x280000..0x28FFFF
            a if a >= Bus24::CRAM_BASE && a < Bus24::CRAM_BASE + 0x10000 => {
                let offset = a - Bus24::CRAM_BASE;
                self.vdp.read_cram(offset)
            }
            // Everything else goes through bus
            _ => self.bus.read_u8(addr),
        }
    }

    /// Write to memory with VDP routing
    pub fn write_memory(&mut self, addr: u32, value: u8) {
        let addr = addr & 0x00FFFFFF;

        // Route VDP regions
        match addr {
            // VDP-T registers: 0x100000..0x103FFF
            a if a >= Bus24::VDP_IO_BASE && a < Bus24::VDP_IO_BASE + 0x4000 => {
                let offset = a - Bus24::VDP_IO_BASE;
                // VDP registers are 16-bit, handle byte writes
                // For simplicity, only process writes on even addresses
                if offset & 1 == 0 {
                    let current = self.vdp.read_reg(offset);
                    let new_value = (current & 0xFF00) | (value as u16);
                    self.vdp.write_reg(offset, new_value);
                } else {
                    let current = self.vdp.read_reg(offset - 1);
                    let new_value = (current & 0x00FF) | ((value as u16) << 8);
                    self.vdp.write_reg(offset - 1, new_value);
                }
            }
            // VRAM: 0x200000..0x27FFFF
            a if a >= Bus24::VRAM_BASE && a < Bus24::VRAM_BASE + 0x80000 => {
                let offset = a - Bus24::VRAM_BASE;
                self.vdp.write_vram(offset, value);
            }
            // CRAM: 0x280000..0x28FFFF
            a if a >= Bus24::CRAM_BASE && a < Bus24::CRAM_BASE + 0x10000 => {
                let offset = a - Bus24::CRAM_BASE;
                self.vdp.write_cram(offset, value);
            }
            // Everything else goes through bus
            _ => self.bus.write_u8(addr, value),
        }
    }

    /// Run the emulator for a specified number of frames
    pub fn run_frames(&mut self, num_frames: u64) {
        for _ in 0..num_frames {
            self.step_frame();

            if self.cpu.halted {
                break;
            }
        }
    }

    /// Get current execution statistics
    pub fn stats(&self) -> EmulatorStats {
        EmulatorStats {
            total_cycles: self.cpu.cycles,
            frame_count: self.frame_count,
            pc: self.cpu.pc,
            halted: self.cpu.halted,
        }
    }
}

impl Default for Nexel24 {
    fn default() -> Self {
        Self::new()
    }
}

/// Emulator execution statistics
#[derive(Debug, Clone, Copy)]
pub struct EmulatorStats {
    pub total_cycles: u64,
    pub frame_count: u64,
    pub pc: u32,
    pub halted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emulator_initialization() {
        let emu = Nexel24::new();
        assert_eq!(emu.cpu.pc, 0xFF0000);
        assert_eq!(emu.frame_count, 0);
        assert!(!emu.cpu.halted);
    }

    #[test]
    fn emulator_reset() {
        let mut emu = Nexel24::new();

        // Load a reset vector
        let bios = vec![0x00, 0x04, 0x40]; // Reset vector points to 0x400400
        emu.load_bios(&bios);

        emu.cpu.a = 0x1234;
        emu.frame_count = 10;

        emu.reset();

        assert_eq!(emu.cpu.a, 0);
        assert_eq!(emu.cpu.pc, 0x400400);
        assert_eq!(emu.frame_count, 0);
    }

    #[test]
    fn emulator_load_and_execute() {
        let mut emu = Nexel24::new();

        // Create a simple program: reset vector + LDA #0x1234, HLT
        let mut program = vec![0x03, 0x00, 0xFF]; // Reset vector: 0xFF0003
        program.extend_from_slice(&[
            0x01, 0x34, 0x12, // LDA #0x1234
            0xFF, // HLT
        ]);
        emu.load_bios(&program);
        emu.reset();

        // Execute the program
        emu.step(); // LDA
        assert_eq!(emu.cpu.a, 0x1234);
        assert!(!emu.cpu.halted);

        emu.step(); // HLT
        assert!(emu.cpu.halted);
    }

    #[test]
    fn emulator_step_frame() {
        let mut emu = Nexel24::new();

        // Create a program that does some NOPs then halts
        let mut program = vec![0x03, 0x00, 0xFF]; // Reset vector: 0xFF0003
        program.extend_from_slice(&vec![0x00; 100]); // 100 NOPs
        program.push(0xFF); // HLT
        emu.load_bios(&program);
        emu.reset();

        let initial_cycles = emu.cpu.cycles;
        emu.step_frame();

        // Should have executed many instructions
        assert!(emu.cpu.cycles > initial_cycles);
        assert_eq!(emu.frame_count, 1);
    }

    #[test]
    fn emulator_run_multiple_frames() {
        let mut emu = Nexel24::new();

        // Create a program with infinite loop (BRA -2)
        let mut program = vec![0x03, 0x00, 0xFF]; // Reset vector: 0xFF0003
        program.extend_from_slice(&[
            0x30, 0xFE, // BRA -2 (infinite loop)
        ]);
        emu.load_bios(&program);
        emu.reset();

        emu.run_frames(5);

        assert_eq!(emu.frame_count, 5);
        // Cycles should be approximately 5 * CYCLES_PER_FRAME
        assert!(emu.cpu.cycles >= 5 * Nexel24::CYCLES_PER_FRAME);
    }

    #[test]
    fn emulator_stats() {
        let mut emu = Nexel24::new();

        let mut program = vec![0x03, 0x00, 0xFF]; // Reset vector: 0xFF0003
        program.extend_from_slice(&[0x01, 0x34, 0x12, 0xFF]); // LDA #0x1234, HLT
        emu.load_bios(&program);
        emu.reset();

        emu.step();
        emu.step();

        let stats = emu.stats();
        assert_eq!(stats.total_cycles, 3); // LDA (2) + HLT (1)
        assert!(stats.halted);
    }

    #[test]
    fn emulator_cycles_per_frame_constant() {
        // Verify the constant is calculated correctly
        assert_eq!(Nexel24::CYCLES_PER_FRAME, 307_200);
        assert_eq!(Nexel24::CPU_CLOCK_HZ / Nexel24::TARGET_FPS, 307_200);
    }

    #[test]
    fn emulator_vdp_register_access() {
        let mut emu = Nexel24::new();

        // Write to VDP display control register via memory bus
        emu.write_memory(0x100000, 0x07); // Enable display and all layers
        emu.write_memory(0x100001, 0x00);

        // Read back the value
        let val_lo = emu.read_memory(0x100000);
        let val_hi = emu.read_memory(0x100001);
        let value = (val_lo as u16) | ((val_hi as u16) << 8);

        assert_eq!(value, 0x07);
        // Verify that the VDP received the write by checking it can read the register
        assert_eq!(emu.vdp.read_reg(0), 0x07);
    }

    #[test]
    fn emulator_vdp_vram_access() {
        let mut emu = Nexel24::new();

        // Write to VRAM through memory bus
        emu.write_memory(0x200000, 0x42);
        emu.write_memory(0x200001, 0x43);
        emu.write_memory(0x200002, 0x44);

        // Read back from VRAM
        assert_eq!(emu.read_memory(0x200000), 0x42);
        assert_eq!(emu.read_memory(0x200001), 0x43);
        assert_eq!(emu.read_memory(0x200002), 0x44);
    }

    #[test]
    fn emulator_vdp_cram_access() {
        let mut emu = Nexel24::new();

        // Write palette data to CRAM through memory bus
        emu.write_memory(0x280000, 0x3F); // Red component
        emu.write_memory(0x280001, 0x00); // Green component
        emu.write_memory(0x280002, 0x00); // Blue component

        // Read back from CRAM
        assert_eq!(emu.read_memory(0x280000), 0x3F);
        assert_eq!(emu.read_memory(0x280001), 0x00);
        assert_eq!(emu.read_memory(0x280002), 0x00);
    }

    #[test]
    fn emulator_vdp_timing_integration() {
        let mut emu = Nexel24::new();

        // Enable VDP display
        emu.vdp.set_display_enable(true);

        let initial_frame_count = emu.vdp.frame_count();

        // Run for one frame
        emu.step_frame();

        // VDP should have advanced
        assert!(emu.vdp.frame_count() > initial_frame_count || emu.cpu.halted);
    }
}
