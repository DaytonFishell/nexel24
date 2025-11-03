# Nexel-24 Game Console Emulator

This is a Rust implementation of the **Nexel-24 (HX-1)**, a fictional retro game console featuring a 24-bit address space, 16-bit data path, and modular coprocessor architecture. The project includes both the hardware emulator and the **Baseplate VM** runtime for safe game execution.

## Architecture Overview

### Core Components (src/ modules planned)
- **CPU (HXC-24)**: 18.432MHz, 2-stage pipeline, memory-mapped coprocessors
- **VDP-T**: Tile/sprite GPU with affine transforms, 512KB VRAM
- **VLU-24**: Vector math coprocessor for 3D operations
- **APU-6**: 6-channel audio processor (PCM, FM, wavetable, noise)
- **Baseplate VM**: Bytecode interpreter with cooperative fibers and capability-based security

### Memory Map (24-bit addressing)
- `0x000000-0x0FFFFF`: WorkRAM (1MB)
- `0x100000-0x103FFF`: VDP-T registers/VRAM
- `0x108000-0x10BFFF`: VLU-24 coprocessor
- `0x10C000-0x10FFFF`: APU-6 coprocessor
- Cartridge ROM mapped in upper address space

## Development Patterns

### Feature Flags
- `dx`: Enables DX RAM and faster DMA paths for performance testing
- `debug-ui`: SDL2/egui overlays for debugging (VRAM viewer, sprite inspector)
- `fast-math`: VLU approximations for speed vs accuracy trade-offs
- `serde-spec`: JSON/YAML serialization for loading specification files

### Project Structure Philosophy
Follow the modular coprocessor design: each major component (`cpu/`, `vdp/`, `vlu/`, `apu/`, `vm/`) should be self-contained with clear interfaces. The `core/` module provides shared bus/timing infrastructure.

### Timing and Cycles
All operations must specify cycle counts per the specification. CPU instructions are 1-4 cycles, DMA is cycle-accurate, and coprocessors have specific latencies (VLU: 12 cycles for transforms, APU: variable based on DSP complexity).

### Baseplate VM Integration
- Bytecode files use `.bpx` extension with 3-byte instruction alignment
- 32-bit tagged values (8-bit type tag + 24-bit payload)
- Handle-based API for safe hardware access (TexHandle, SpriteHandle, etc.)
- Cooperative fibers with 65536 instruction quota per frame

## Key Implementation Details

### Memory Access Patterns
24-bit addresses require special handling - avoid standard Rust pointer arithmetic. Use bus abstraction in `core/bus.rs` for all memory access to maintain timing accuracy and coprocessor routing.

### Interrupt Priority
Hardware interrupts follow strict priority: `NMI > HBLANK > DMA_DONE > VLU_DONE > APU_BUF_EMPTY > TIMER0-2 > PAD_EVENT > SWI`. Implement as priority queue in CPU scheduler.

### Build and Test Commands
```bash
# Basic build
cargo build

# With debug UI for development
cargo build --features debug-ui

# Performance testing with DX features
cargo build --features dx,fast-math

# Load specification files
cargo build --features serde-spec

# Run hardware component tests
cargo test cpu_ops
cargo test dma_timing
cargo test vm_verifier
```

### Debugging Tools
When `debug-ui` feature is enabled, emulator provides:
- VRAM/CRAM memory viewers
- Sprite layer inspector
- VLU operation queue monitor
- UART console output
- Cycle-accurate timing display

## File Format Specifications
- Cartridge files: `.nxl` (NexOS loader format)
- Baseplate modules: `.bpx` (bytecode with constant pool)
- Save data: EEPROM (8-256KB) or Flash (up to 8MB)
- Asset pipeline: `.pxi` (images), `.pxa` (audio), `.pxm` (maps)

Reference the comprehensive specs in `nexel24_spec.json` and `baseplate_bytecode_schema.yaml` for exact timing, instruction formats, and hardware behavior.