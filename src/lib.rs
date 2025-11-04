//! Nexel-24 (HX-1) Game Console Emulator
//!
//! This library provides core emulation components for the Nexel-24 console,
//! including CPU, memory bus, and coprocessor subsystems.

pub mod apu;
pub mod bytecode;
pub mod core;
pub mod cpu;
pub mod emulator;
pub mod vdp;
pub mod vlu;
pub mod vm; // <--- added module declaration

pub use apu::Apu;
// Re-export commonly used types
pub use core::Bus24;
pub use cpu::Cpu;
pub use emulator::{EmulatorStats, Nexel24};
pub use vdp::Vdp;
pub use vlu::Vlu;
pub use vm::BaseplateVm;
