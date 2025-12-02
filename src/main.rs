// Copyright (C) 2025 Dayton Fishell
// Nexel-24 Game Console Emulator
// This file is part of Nexel-24.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version. See the LICENSE file in the project root for details.
// SPDX-License-Identifier: GPL-3.0-or-later

// A simple demo program for the Nexel-24 emulator.
use nexel_core::Nexel24;

fn main() {
    env_logger::init();

    println!("Nexel-24 Emulator v0.1.0");
    println!("========================");
    println!();

    // Create a new emulator instance
    let mut emulator = Nexel24::new();

    // Create a simple demo program
    // Reset vector points to 0xFF0003, then:
    // LDA #0x1234, STA $1000, LDX #0x5678, STX $1002, HLT
    let demo_program = vec![
        0x03, 0x00, 0xFF, // Reset vector: 0xFF0003
        0x01, 0x34, 0x12, // LDA #0x1234
        0x02, 0x00, 0x10, 0x00, // STA $001000
        0x03, 0x78, 0x56, // LDX #0x5678
        0x04, 0x02, 0x10, 0x00, // STX $001002
        0xFF, // HLT
    ];

    println!("Loading demo program into BIOS...");
    emulator.load_bios(&demo_program);
    emulator.reset();

    println!("Initial state:");
    println!("  PC: 0x{:06X}", emulator.cpu.pc);
    println!("  A:  0x{:04X}", emulator.cpu.a);
    println!("  X:  0x{:04X}", emulator.cpu.x);
    println!();

    println!("Executing demo program...");

    // Execute instruction by instruction
    let mut instruction_count = 0;
    while !emulator.cpu.halted && instruction_count < 100 {
        emulator.step();
        instruction_count += 1;
    }

    println!("Execution complete!");
    println!();

    let stats = emulator.stats();
    println!("Final state:");
    println!("  PC:          0x{:06X}", stats.pc);
    println!("  A:           0x{:04X}", emulator.cpu.a);
    println!("  X:           0x{:04X}", emulator.cpu.x);
    println!("  Cycles:      {}", stats.total_cycles);
    println!("  Instructions: {}", instruction_count);
    println!("  Halted:      {}", stats.halted);
    println!();

    // Check memory at 0x1000
    let value_at_1000 = emulator.bus.read_u16(0x1000);
    let value_at_1002 = emulator.bus.read_u16(0x1002);
    println!("Memory contents:");
    println!("  [0x001000]: 0x{:04X}", value_at_1000);
    println!("  [0x001002]: 0x{:04X}", value_at_1002);
    println!();

    if value_at_1000 == 0x1234 && value_at_1002 == 0x5678 {
        println!("✓ Demo program executed successfully!");
    } else {
        println!("✗ Demo program did not execute as expected.");
    }
}
