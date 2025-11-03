//! Nexel-24 (HX-1) Game Console Emulator
//!
//! This library provides core emulation components for the Nexel-24 console,
//! including CPU, memory bus, and coprocessor subsystems.

pub mod core;
pub mod cpu;
pub mod vdp;
pub mod vlu;
pub mod apu;
pub mod vm;
pub mod emulator;

// Re-export commonly used types
pub use core::Bus24;
pub use cpu::Cpu;
pub use vdp::Vdp;
pub use vlu::Vlu;
pub use apu::Apu;
pub use vm::BaseplateVm;
pub use emulator::{Nexel24, EmulatorStats};
