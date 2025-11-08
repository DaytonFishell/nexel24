# Nexel-24 Emulator (nexel24)

This repository contains a Rust emulator implementation for the fictional Nexel-24 (HX-1) console and the Baseplate VM runtime. The emulator includes a working CPU, full memory bus implementation, VDP-T graphics coprocessor, and basic execution framework.

## Features

- **HXC-24 CPU**: 18.432MHz, 24-bit addressing, 16-bit data path
- **Full Memory Map**: WorkRAM, ExpandedRAM, I/O, VRAM, CRAM, Cartridge ROM/Save, BIOS
- **18+ CPU Instructions**: Load/Store, ALU operations, branching, subroutines, interrupts
- **Interrupt Handling**: Priority-based interrupt system with NMI support
- **Cycle-Accurate Timing**: Proper cycle counting for all operations
- **Frame-Based Execution**: Execute programs at 60 FPS with accurate timing
- **VDP-T Graphics Coprocessor**: Tile/sprite GPU with register interface and basic rendering

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

### Run the VDP-T demo

```bash
cargo run --example vdp_demo
```

This demonstrates the VDP-T graphics coprocessor, including display modes, palette loading, sprite configuration, and VRAM/CRAM access.

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

## VDP-T Graphics Coprocessor

The VDP-T (Tile/Sprite GPU) is the Nexel-24's graphics coprocessor, providing tile-based and sprite-based rendering capabilities.

### Implemented Features

- **Display Modes**: Native 384x288, 320x240, and 256x224 resolutions
- **Memory-Mapped Registers**: Full register interface at 0x100000-0x103FFF
- **VRAM**: 512KB video memory at 0x200000-0x27FFFF for tiles and sprite data
- **CRAM**: 64KB palette memory at 0x280000-0x28FFFF (18-bit RGB666 colors)
- **Background Layers**:
  - BG0: Affine-capable background with transformation support (planned)
  - BG1: Static tilemap background with scrolling (implemented)
- **Sprite System**: 
  - Up to 128 sprites on screen
  - Hardware limit of 64 sprites per scanline
  - Sizes: 8x8, 16x16, 32x32, 64x64
  - Per-sprite attributes: palette, flip H/V, priority
- **Rendering**: Software framebuffer rendering with backdrop color support
- **Timing**: Cycle-accurate scanline timing with VBLANK/HBLANK tracking
- **Palette System**: 16 palettes with 256 colors each (RGB666 format)

### VDP-T Registers

| Offset | Name | Description |
|--------|------|-------------|
| 0x0000 | DISPCTL | Display control (enable display, layers, IRQs) |
| 0x0002 | DISPSTAT | Display status (VBLANK, HBLANK, DMA busy) |
| 0x0004 | VCOUNT | Current scanline (0-287) |
| 0x0006 | HCOUNT | Horizontal position |
| 0x0010 | BG0CTL | Background 0 control |
| 0x0012 | BG0SCROLLX | Background 0 scroll X |
| 0x0014 | BG0SCROLLY | Background 0 scroll Y |
| 0x0030 | BG1CTL | Background 1 control |
| 0x0032 | BG1SCROLLX | Background 1 scroll X |
| 0x0034 | BG1SCROLLY | Background 1 scroll Y |
| 0x0070 | DMASRC | DMA source address |
| 0x0074 | DMADEST | DMA destination address |
| 0x0078 | DMALEN | DMA transfer length |
| 0x007A | DMACTL | DMA control/start |

### Example: Using the VDP-T

```rust
use nexel_core::vdp::{Vdp, SpriteAttr};

let mut vdp = Vdp::new();

// Configure display
vdp.set_display_mode(320, 240);
vdp.set_display_enable(true);
vdp.set_layer_enable(true, true, true);

// Load palette
let colors = vec![
    (0x00, 0x00, 0x00), // Black (transparent)
    (0x3F, 0x00, 0x00), // Red
    (0x00, 0x3F, 0x00), // Green
    (0x00, 0x00, 0x3F), // Blue
];
vdp.load_palette(0, &colors);

// Create and configure a sprite
let sprite = SpriteAttr {
    y_pos: 100,
    x_pos: 100,
    tile_index: 0,
    attr: 0x8000, // Enabled, 8x8 size
};
vdp.set_sprite(0, sprite);

// Step VDP timing and render frames
vdp.step(Vdp::CYCLES_PER_SCANLINE * Vdp::SCANLINES_PER_FRAME as u64);
```

### Example: Using the VLU-24

```rust
use nexel_core::cpu::Cpu;
use nexel_core::vlu::{Vlu, VluJob, VluResult};

let mut cpu = Cpu::new();
let mut vlu = Vlu::new();

// Load vector and matrix registers
vlu.set_vector(0, [1.0, 0.0, 0.0])?;
vlu.set_matrix(0, [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]])?;

// Rotate vector around Z by 90 degrees
match vlu.compute(
    &mut cpu,
    VluJob::Transform {
        dest: 1,
        vec: 0,
        matrix: 0,
    },
) {
    Ok(VluResult::Vector(rotated)) => assert_eq!(rotated, [0.0, 1.0, 0.0]),
    Ok(_) => unreachable!(),
    Err(err) => panic!("VLU error: {err}"),
}
```

## Repository Structure

```
src/
├── core/
│   ├── mod.rs          - Core module exports
│   └── bus.rs          - 24-bit memory bus implementation
├── cpu.rs              - HXC-24 CPU implementation
├── vdp.rs              - VDP-T GPU implementation
├── vlu.rs              - VLU-24 vector coprocessor
├── apu.rs              - APU-6 audio processor (stub)
├── vm.rs               - Baseplate VM (stub)
├── bytecode.rs         - Baseplate bytecode module loader (stub)
├── emulator.rs         - Main emulator integration
├── lib.rs              - Library exports
└── main.rs             - Demo program

examples/
└── vdp_demo.rs         - VDP-T demonstration program
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
- **11 Emulator tests**: Integration, frame timing, execution flow, VDP integration
- **8 VDP tests**: Register access, display modes, VRAM/CRAM, palette loading, sprite attributes, timing

Run all tests with:

```bash
cargo test
```

Run specific test suites:

```bash
cargo test bus      # Bus tests only
cargo test cpu      # CPU tests only
cargo test emulator # Emulator tests only
cargo test vdp      # VDP tests only
```

## Next Steps

- [x] Implement interrupt handling (NMI, IRQ, timers)
- [x] Add VDP-T register interface and basic rendering
- [x] Complete VDP-T affine transformation for BG0 layer
- [x] Implement VLU-24 vector operations
- [ ] Add APU-6 audio channel control
- [ ] Baseplate VM bytecode interpreter
- [ ] Add continuous integration workflow
- [ ] Implement more CPU addressing modes
- [ ] Implement VDP-T DMA transfers

## Specifications

See `nexel24_spec.json`, `baseplate_bytecode_schema.yaml`, and `docs/VLU_REFERENCE.md` for detailed hardware specifications and subsystem references.

## License

This is a personal project for educational purposes.
