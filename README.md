# Nexel-24 Emulator (nexel24)

This repository contains a Rust emulator implementation for the fictional Nexel-24 (HX-1) console and the Baseplate VM runtime. The emulator includes a working CPU, full memory bus implementation, and basic execution framework.

## Features

- **HXC-24 CPU**: 18.432MHz, 24-bit addressing, 16-bit data path
- **Full Memory Map**: WorkRAM, ExpandedRAM, I/O, VRAM, CRAM, Cartridge ROM/Save, BIOS
- **18+ CPU Instructions**: Load/Store, ALU operations, branching, subroutines, interrupts
- **Interrupt Handling**: Priority-based interrupt system with NMI support
- **Cycle-Accurate Timing**: Proper cycle counting for all operations
- **Frame-Based Execution**: Execute programs at 60 FPS with accurate timing

## Quick Start

### Build the project

```bash
cargo build
```

### Run tests

```bash
cargo test
```

### Run the demo program

```bash
cargo run
```

This will execute a simple demo program that demonstrates the CPU's capabilities.

## Usage

### As a Library

```rust
use nexel_core::Nexel24;

fn main() {
    // Create a new emulator
    let mut emulator = Nexel24::new();
    
    // Load a program into BIOS
    let program = vec![
        0x03, 0x00, 0xFF,       // Reset vector: 0xFF0003
        0x01, 0x34, 0x12,       // LDA #0x1234
        0xFF,                   // HLT
    ];
    emulator.load_bios(&program);
    emulator.reset();
    
    // Execute until halt
    while !emulator.cpu.halted {
        emulator.step();
    }
    
    println!("A register: 0x{:04X}", emulator.cpu.a);
}
```

### Memory Map

```
0x000000..0x00FFFF:  WorkRAM (64KB) - Primary stack/heap
0x010000..0x03FFFF:  ExpandedRAM (192KB)
0x100000..0x10FFFF:  I/O (64KB) - Memory-mapped coprocessors
0x200000..0x27FFFF:  VRAM (512KB)
0x280000..0x28FFFF:  CRAM (64KB)
0x400000..0x9FFFFF:  CartROM (6MB)
0xA00000..0xA3FFFF:  CartSave (256KB)
0xFF0000..0xFFFFFF:  BIOS (64KB)
```

## Implemented CPU Instructions

| Opcode | Mnemonic | Description | Cycles |
|--------|----------|-------------|--------|
| 0x00   | NOP      | No operation | 1 |
| 0x01   | LDA #imm | Load accumulator (immediate) | 2 |
| 0x02   | STA addr | Store accumulator (absolute) | 3 |
| 0x03   | LDX #imm | Load X register (immediate) | 2 |
| 0x04   | STX addr | Store X register (absolute) | 3 |
| 0x05   | LDY #imm | Load Y register (immediate) | 2 |
| 0x06   | STY addr | Store Y register (absolute) | 3 |
| 0x10   | ADD #imm | Add to accumulator | 2 |
| 0x11   | SUB #imm | Subtract from accumulator | 2 |
| 0x12   | AND #imm | Logical AND | 2 |
| 0x13   | OR #imm  | Logical OR | 2 |
| 0x14   | XOR #imm | Logical XOR | 2 |
| 0x20   | JMP addr | Jump absolute | 3 |
| 0x21   | JSR addr | Jump to subroutine | 5 |
| 0x22   | RTS      | Return from subroutine | 4 |
| 0x30   | BRA rel  | Branch always | 2 |
| 0x31   | BEQ rel  | Branch if equal (zero set) | 2-3 |
| 0x32   | BNE rel  | Branch if not equal (zero clear) | 2-3 |
| 0x40   | SEI      | Set interrupt disable | 1 |
| 0x41   | CLI      | Clear interrupt disable | 1 |
| 0x42   | RTI      | Return from interrupt | 5 |
| 0xFF   | HLT      | Halt processor | 1 |

## Repository Structure

```
src/
├── core/
│   ├── mod.rs          - Core module exports
│   └── bus.rs          - 24-bit memory bus implementation
├── cpu.rs              - HXC-24 CPU implementation
├── vdp.rs              - VDP-T GPU (stub)
├── vlu.rs              - VLU-24 vector coprocessor (stub)
├── apu.rs              - APU-6 audio processor (stub)
├── vm.rs               - Baseplate VM (stub)
├── emulator.rs         - Main emulator integration
├── lib.rs              - Library exports
└── main.rs             - Demo program
```

## Feature Flags

- `dx` — Enable DX RAM and faster DMA paths for performance testing
- `debug-ui` — Enable SDL2/egui debug overlays (planned)
- `fast-math` — Enable VLU approximations for speed vs accuracy trade-offs
- `serde-spec` — Enable JSON/YAML serialization for loading specification files

## Testing

The project includes comprehensive tests:

- **14 Bus tests**: Memory region access, addressing, read-only regions
- **25 CPU tests**: Instruction execution, flags, cycle counting, interrupt handling
- **7 Emulator tests**: Integration, frame timing, execution flow

Run all tests with:

```bash
cargo test
```

Run specific test suites:

```bash
cargo test bus      # Bus tests only
cargo test cpu      # CPU tests only
cargo test emulator # Emulator tests only
```

## Next Steps

- [x] Implement interrupt handling (NMI, IRQ, timers)
- [ ] Add VDP-T register interface and basic rendering
- [ ] Implement VLU-24 vector operations
- [ ] Add APU-6 audio channel control
- [ ] Baseplate VM bytecode interpreter
- [ ] Add continuous integration workflow
- [ ] Implement more CPU addressing modes
- [ ] Add DMA support

## Specifications

See `nexel24_spec.json` and `baseplate_bytecode_schema.yaml` for detailed hardware specifications and file formats.

## License

This is a personal project for educational purposes.
