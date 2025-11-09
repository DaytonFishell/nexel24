# VDP-T Quick Reference

## Register Map (Base: 0x100000)

| Offset | Name           | R/W | Description                      |
|--------|----------------|-----|----------------------------------|
| 0x0000 | DisplayControl | R/W | Master display control flags     |
| 0x0002 | DisplayStatus  | R   | Display status (VBLANK, HBLANK)  |
| 0x0004 | VCount         | R   | Current scanline (0-287)         |
| 0x0006 | HCount         | R   | Horizontal position              |
| 0x0010 | BG0Control     | R/W | Background 0 control             |
| 0x0012 | BG0ScrollX     | R/W | BG0 X scroll (-32768 to 32767)   |
| 0x0014 | BG0ScrollY     | R/W | BG0 Y scroll                     |
| 0x0016 | BG0AffineA     | R/W | Affine matrix A (pa, 8.8 fixed) |
| 0x0018 | BG0AffineB     | R/W | Affine matrix B (pb, 8.8 fixed) |
| 0x001A | BG0AffineC     | R/W | Affine matrix C (pc, 8.8 fixed) |
| 0x001C | BG0AffineD     | R/W | Affine matrix D (pd, 8.8 fixed) |
| 0x001E | BG0RefX        | R/W | Reference point X (24-bit)       |
| 0x0020 | BG0RefX+2      | R/W | Reference point X high byte      |
| 0x0022 | BG0RefY        | R/W | Reference point Y (24-bit)       |
| 0x0024 | BG0RefY+2      | R/W | Reference point Y high byte      |
| 0x0026 | BG0TilemapAddr | R/W | BG0 tilemap base address         |
| 0x0030 | BG1Control     | R/W | Background 1 control             |
| 0x0032 | BG1ScrollX     | R/W | BG1 X scroll                     |
| 0x0034 | BG1ScrollY     | R/W | BG1 Y scroll                     |
| 0x0036 | BG1TilemapAddr | R/W | Tilemap base address in VRAM     |
| 0x0070 | DmaSource      | R/W | DMA source address (24-bit)      |
| 0x0074 | DmaDestination | R/W | DMA destination address (24-bit) |
| 0x0078 | DmaLength      | R/W | DMA transfer length              |
| 0x007A | DmaControl     | R/W | DMA control (bit 15 = start)     |

## DisplayControl Flags (0x0000)

| Bit | Name           | Description                   |
|-----|----------------|-------------------------------|
| 0   | ENABLE         | Master display enable         |
| 1   | BG0_ENABLE     | Enable background layer 0     |
| 2   | BG1_ENABLE     | Enable background layer 1     |
| 3   | SPRITE_ENABLE  | Enable sprite rendering       |
| 4   | POLYGON_ENABLE | Enable polygon rendering      |
| 8   | HBLANK_IRQ     | Enable HBLANK interrupt       |
| 9   | VBLANK_IRQ     | Enable VBLANK interrupt       |
| 10  | LINECMP_IRQ    | Enable line compare interrupt |
| 12  | MODE_320x240   | 320x240 display mode          |
| 13  | MODE_256x224   | 256x224 display mode          |

## DisplayStatus Flags (0x0002)

| Bit | Name         | Description                |
|-----|--------------|----------------------------|
| 0   | VBLANK       | Currently in VBLANK period |
| 1   | HBLANK       | Currently in HBLANK period |
| 2   | LINECMP      | Line compare match         |
| 3   | DMA_BUSY     | DMA transfer in progress   |
| 4   | CMDLIST_BUSY | Command list processing    |

## BgControl Flags

| Bit   | Name       | Description                  |
|-------|------------|------------------------------|
| 0     | ENABLE     | Enable this background layer |
| 4-5   | PRIORITY   | Layer priority (0-3)         |
| 6     | MOSAIC     | Enable mosaic effect         |
| 7     | COLOR_256  | 256-color mode (vs 16-color) |
| 8     | AFFINE     | Affine transformation mode   |
| 9     | WRAPAROUND | Wraparound at edges          |
| 10-11 | SIZE       | Tilemap size (32/64/128)     |

## Sprite Attributes (OAM Entry = 8 bytes)

| Offset | Size | Name       | Description            |
|--------|------|------------|------------------------|
| 0      | 2    | y_pos      | Y position (0-511)     |
| 2      | 2    | x_pos      | X position (0-511)     |
| 4      | 2    | tile_index | Tile index in VRAM     |
| 6      | 2    | attr       | Attributes (see below) |

### Attribute Bits

| Bit   | Name     | Description                            |
|-------|----------|----------------------------------------|
| 15    | ENABLE   | Sprite enabled                         |
| 14    | FLIP_V   | Vertical flip                          |
| 13    | FLIP_H   | Horizontal flip                        |
| 12-10 | PRIORITY | Priority level (0-3)                   |
| 11-8  | PALETTE  | Palette index (0-15)                   |
| 1-0   | SIZE     | Size: 0=8x8, 1=16x16, 2=32x32, 3=64x64 |

## Memory Regions

- **VRAM**: 0x200000 - 0x27FFFF (512KB)
    - Tile data, sprite data, tilemaps

- **CRAM**: 0x280000 - 0x28FFFF (64KB)
    - 16 palettes × 256 colors × 3 bytes (RGB666)

## Timing Constants

- **Cycles per scanline**: 1024
- **Scanlines per frame**: 288
- **VBLANK start**: Scanline 240
- **HBLANK trigger**: H-count >= 768

## Display Modes

| Mode   | Width | Height | Notes              |
|--------|-------|--------|--------------------|
| Native | 384   | 288    | Default mode       |
| Mode 1 | 320   | 240    | Common 4:3 mode    |
| Mode 2 | 256   | 224    | Retro console mode |

## Hardware Limits

- **Maximum sprites on screen**: 128
- **Maximum sprites per scanline**: 64
- **Color depth**: 18-bit RGB666 (262,144 colors)
- **Palettes**: 16 palettes of 256 colors each
- **Tile sizes**: 8×8 pixels, 8 bits per pixel

## API Quick Reference

```rust
// Initialize VDP
let mut vdp = Vdp::new();

// Set display mode
vdp.set_display_mode(320, 240);
vdp.set_display_enable(true);
vdp.set_layer_enable(true, true, true); // BG0, BG1, Sprites

// Load palette
vdp.load_palette(0, & [(r, g, b), ...]);

// Load tile data
vdp.load_tile_data(offset, & tile_bytes);

// Configure sprite
let sprite = SpriteAttr {
y_pos: 100,
x_pos: 100,
tile_index: 0,
attr: 0x8101, // Enabled, palette 1, 16x16
};
vdp.set_sprite(0, sprite);

// Access framebuffer
let fb = vdp.framebuffer(); // &[u32]
```

## Example: Simple Tile Setup

```rust
// Create 8x8 tile (64 pixels)
let mut tile = vec![0u8; 64];
for y in 0..8 {
for x in 0..8 {
tile[y * 8 + x] = if x < 4 { 1 } else { 2 };
}
}
vdp.load_tile_data(0, & tile);

// Set up tilemap entry
// Format: [flip_v][flip_h][palette:4][tile_index:10]
let tile_entry = 0x0000u16; // Tile 0, palette 0, no flip
vdp.write_vram(tilemap_addr, (tile_entry & 0xFF) as u8);
vdp.write_vram(tilemap_addr + 1, (tile_entry >> 8) as u8);
```

## Color Format

RGB666 (18-bit color):

- Each component: 6 bits (0-63)
- CRAM storage: 3 bytes per color (R, G, B)
- Internal conversion to RGB888 for framebuffer

```rust
// Set a color in CRAM
let palette_offset = (palette_idx * 256 * 3) + (color_idx * 3);
vdp.write_cram(palette_offset + 0, red & 0x3F);
vdp.write_cram(palette_offset + 1, green & 0x3F);
vdp.write_cram(palette_offset + 2, blue & 0x3F);
```

