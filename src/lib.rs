// Copyright (C) 2025 Dayton Fishell
// Nexel-24 Game Console Emulator
// This file is part of Nexel-24.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version. See the LICENSE file in the project root for details.
// SPDX-License-Identifier: GPL-3.0-or-later

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
