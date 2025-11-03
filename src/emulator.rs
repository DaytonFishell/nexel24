//! Main Nexel-24 emulator integration
//!
//! This module provides the main emulator struct that coordinates the CPU,
//! memory bus, and coprocessors.

use crate::core::Bus24;
use crate::cpu::Cpu;
use crate::vdp::Vdp;
use crate::vlu::Vlu;
use crate::apu::Apu;
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
        Self {
            cpu: Cpu::new(),
            bus: Bus24::new(),
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

    /// Execute a single CPU instruction
    pub fn step(&mut self) {
        self.cpu.step(&mut self.bus);
    }

    /// Execute instructions for one frame (approximately 307,200 cycles at 60 FPS)
    pub fn step_frame(&mut self) {
        let start_cycles = self.cpu.cycles;
        let target_cycles = start_cycles + self.target_cycles_per_frame;

        while self.cpu.cycles < target_cycles && !self.cpu.halted {
            self.cpu.step(&mut self.bus);
        }

        self.frame_count += 1;
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
            0xFF,             // HLT
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
}
