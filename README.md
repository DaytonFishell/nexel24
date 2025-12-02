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

## Native Assembly (NRAW)

NRAW is the Nexel-24's native assembly language. The emulator includes a comprehensive assembler via `nexel_core::nraw::assemble` that supports:

- **Complete instruction set**: All core and extension instructions
- **Label declarations and references**: For code organization and jumps
- **Multiple addressing modes**: Immediate (#), absolute, and relative
- **Register operations**: MOV, INC, DEC with register names (A, X, Y, SP, R0-R7)

```rust
use nexel_core::{Nexel24, nraw::assemble};

let mut emulator = Nexel24::new();
let program = assemble(r#"
start:
    LDA #0x1234
    ADD #0x0010
    STA result
    HLT
result:
    NOP
"#).expect("assemble");

let mut bios = vec![0xFF; 0x10000];
bios[..program.bytes.len()].copy_from_slice(&program.bytes);
emulator.load_bios(&bios);
emulator.reset();
```

Use `program.labels` to inspect branch targets or data offsets when you need to bake jump tables or install interrupt vectors.

### NRAW Calling Convention

The emulator follows a standard calling convention for function calls. See [`docs/CALLING_CONVENTION.md`](docs/CALLING_CONVENTION.md) for details on:
- Register usage (caller-saved vs callee-saved)
- Parameter passing (first 4 in R0-R3, rest on stack)
- Return values (A register for 16-bit, A:X for 32-bit)
- Stack frame layout

## Nexel-24 BIOS

The emulator ships with a functional BIOS image built from NRAW that provides:

- **System initialization**: Sets up VDP and interrupts
- **Interrupt vector table**: Pre-configured handlers for all interrupt sources
- **System call interface**: BIOS functions accessible via JSR 0xFF0100
- **Cartridge boot**: Automatically detects and boots cartridge ROMs

```rust
use nexel_core::Nexel24;

let mut emulator = Nexel24::new();
emulator.load_default_bios();
emulator.reset();
// BIOS initializes system and boots cartridge if present
```

### BIOS Features

The built-in BIOS provides:
- Interrupt handler framework (NMI, HBLANK, VLU_DONE, APU_BUF_EMPTY, etc.)
- System calls for common operations:
  - Syscall 0: Get BIOS version
  - Syscall 1: Wait for VBlank
  - Syscall 2: Delay loop
- Automatic cartridge detection and boot (jumps to 0x400000)
- VDP initialization with display enabled

See [`docs/BIOS_API.md`](docs/BIOS_API.md) for complete BIOS API documentation.

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

```text
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

The HXC-24 CPU now supports a comprehensive instruction set including core and extension instructions:

### Core Instructions

| Opcode | Mnemonic | Description | Cycles |
|--------|----------|-------------|--------|
| 0x00   | NOP      | No operation | 1 |
| 0x01   | LDA #imm | Load accumulator (immediate) | 2 |
| 0x02   | STA addr | Store accumulator (absolute) | 3 |
| 0x03   | LDX #imm | Load X register (immediate) | 2 |
| 0x04   | STX addr | Store X register (absolute) | 3 |
| 0x05   | LDY #imm | Load Y register (immediate) | 2 |
| 0x06   | STY addr | Store Y register (absolute) | 3 |
| 0x07   | LDA addr | Load accumulator (absolute) | 4 |
| 0x08   | LDX addr | Load X register (absolute) | 4 |
| 0x09   | LDY addr | Load Y register (absolute) | 4 |
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
| 0x44   | COP #imm | Coprocessor instruction | 3 |
| 0xFF   | HLT      | Halt processor | 1 |

### Extension Instructions

| Opcode | Mnemonic | Description | Cycles |
|--------|----------|-------------|--------|
| 0x15   | MUL #imm | Multiply (result in A:X) | 4 |
| 0x16   | DIV #imm | Divide (quotient in A, remainder in X) | 2-12 |
| 0x17   | MOV reg  | Move between registers | 2 |
| 0x18   | INC reg  | Increment register | 2 |
| 0x19   | DEC reg  | Decrement register | 2 |
| 0x1A   | BIT #imm | Test bits | 2 |
| 0x1B   | BSET #imm| Set bits in accumulator | 2 |
| 0x1C   | BCLR #imm| Clear bits in accumulator | 2 |
| 0x33   | BCS rel  | Branch if carry set | 2-3 |
| 0x34   | BCC rel  | Branch if carry clear | 2-3 |
| 0x35   | BMI rel  | Branch if minus/negative | 2-3 |
| 0x36   | BPL rel  | Branch if plus/positive | 2-3 |
| 0x37   | BVS rel  | Branch if overflow set | 2-3 |
| 0x38   | BVC rel  | Branch if overflow clear | 2-3 |
| 0x43   | WFI      | Wait for interrupt | 1 |

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

```text
src/
├── core/
│   ├── mod.rs          - Core module exports
│   └── bus.rs          - 24-bit memory bus implementation
├── cpu.rs              - HXC-24 CPU implementation
├── vdp.rs              - VDP-T GPU implementation
├── vlu.rs              - VLU-24 vector coprocessor
├── apu.rs              - APU-6 audio processor (stub)
├── bios.rs             - Built-in BIOS image generator
├── nraw.rs             - NRAW native assembler helper
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
- [x] Add APU-6 audio channel control
- [x] NRAW assembler (native assembly language) with complete instruction set
- [x] NRAW calling convention documentation
- [x] Nexel-24 BIOS with interrupt handlers and system calls
- [x] Complete CPU instruction set (core + extensions)
- [ ] Assembler directives (.org, .db, .dw, .ascii, etc.)
- [ ] Baseplate VM bytecode interpreter
- [ ] Add continuous integration workflow
- [ ] Implement indexed/indirect addressing modes
- [ ] Implement VDP-T DMA transfers

APU-6 channel registers are now routed through the bus layer, and the emulator drives the coprocessor so buffer-empty interrupts reach the CPU interrupt queue.

## Documentation

- **[CALLING_CONVENTION.md](docs/CALLING_CONVENTION.md)**: NRAW calling convention and function call protocol
- **[BIOS_API.md](docs/BIOS_API.md)**: BIOS system calls and interrupt handlers reference
- **[VLU_REFERENCE.md](docs/VLU_REFERENCE.md)**: VLU-24 vector coprocessor operations
- **[VDP_QUICK_REFERENCE.md](docs/VDP_QUICK_REFERENCE.md)**: VDP-T graphics coprocessor quick reference
- **[VDP_IMPLEMENTATION.md](docs/VDP_IMPLEMENTATION.md)**: VDP-T implementation details

## Specifications

See `nexel24_spec.json`, `baseplate_bytecode_schema.yaml`, and documentation files for detailed hardware specifications and subsystem references.

## License

This is a personal project for educational purposes.
