# Nexel-24 Emulator (nexel24)

This repository contains a Rust emulator implementation for the fictional Nexel-24 (HX-1) console and the Baseplate VM runtime. The project is in early development and currently includes core scaffolding and a 24-bit address Bus abstraction.

Quick start

1. Build the project:

```powershell
cd 'f:\Eager Dev\nexel24'
cargo build
```

2. Run tests:

```powershell
cargo test
```

Repository structure (early)

- `src/` - crate source
  - `core/` - core infrastructure (bus, timing)
  - `cpu.rs` - CPU subsystem (stub)
  - `vdp.rs` - VDP-T GPU (stub)
  - `vlu.rs` - Vector coprocessor (stub)
  - `apu.rs` - Audio processor (stub)
  - `vm.rs` - Baseplate VM (stub)

Feature flags

- `dx` — enable DX RAM / DMA experimental code
- `debug-ui` — enable SDL2/egui debug overlays (optional)
- `fast-math` — VLU approximations
- `serde-spec` — enables serde for spec loading

Next steps

- Implement `core::bus` tests and expand mapping to include VDP/APU regions
- Define CPU instruction set and begin implementing decode + execution tests
- Add continuous integration workflow to run builds and tests

See `nexel24_spec.json` and `baseplate_bytecode_schema.yaml` for detailed specifications and file formats.
