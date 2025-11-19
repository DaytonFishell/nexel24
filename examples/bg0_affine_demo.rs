//! BG0 Affine Transformation Demo
//!
//! Demonstrates BG0 affine transformation capabilities:
//! - Identity transformation (no rotation/scaling)
//! - Scaling transformation
//! - Rotation simulation
//! - Non-affine mode (simple scrolling)

use nexel_core::vdp::{BgControl, Vdp, VdpRegister};

fn main() {
    println!("Nexel-24 BG0 Affine Transformation Demo");
    println!("========================================\n");

    // Create VDP instance
    let mut vdp = Vdp::new();

    // Configure display
    println!("Configuring display...");
    vdp.set_display_mode(320, 240);
    vdp.set_layer_enable(true, false, false); // Only BG0 enabled
    vdp.set_display_enable(true);

    let (width, height) = vdp.display_dimensions();
    println!("  Display mode: {}x{}", width, height);

    // Load a simple palette
    println!("\nLoading palette...");
    let colors = vec![
        (0x00, 0x00, 0x00), // 0: Black (transparent)
        (0x3F, 0x00, 0x00), // 1: Red
        (0x00, 0x3F, 0x00), // 2: Green
        (0x00, 0x00, 0x3F), // 3: Blue
        (0x3F, 0x3F, 0x00), // 4: Yellow
        (0x3F, 0x00, 0x3F), // 5: Magenta
        (0x00, 0x3F, 0x3F), // 6: Cyan
        (0x3F, 0x3F, 0x3F), // 7: White
    ];
    vdp.load_palette(0, &colors);
    println!("  Loaded {} colors into palette 0", colors.len());

    // Set backdrop color to dark blue
    vdp.set_backdrop_color(0x00, 0x00, 0x10);
    println!("  Set backdrop color to dark blue");

    // Create some test tiles
    println!("\nCreating test tiles...");

    // Tile 0: Red square with white border
    let mut tile0 = vec![0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            tile0[y * 8 + x] = if x == 0 || x == 7 || y == 0 || y == 7 {
                7 // White border
            } else {
                1 // Red fill
            };
        }
    }
    vdp.load_tile_data(0, &tile0);

    // Tile 1: Green checkerboard
    let mut tile1 = vec![0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            tile1[y * 8 + x] = if (x + y) % 2 == 0 { 2 } else { 0 };
        }
    }
    vdp.load_tile_data(64, &tile1);

    // Tile 2: Blue diagonal pattern
    let mut tile2 = vec![0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            tile2[y * 8 + x] = if x == y { 3 } else { 0 };
        }
    }
    vdp.load_tile_data(128, &tile2);

    println!("  Created 3 test tiles");

    // Set up tilemap
    println!("\nSetting up tilemap...");
    vdp.write_reg(VdpRegister::Bg0TilemapAddr as u32, 0x1000);

    // Create a simple pattern in the tilemap
    for ty in 0..32 {
        for tx in 0..32 {
            let tile_index = ((tx + ty) % 3) as u16; // Cycle through tiles 0, 1, 2
            let tile_entry = tile_index; // No palette selection, no flip
            let offset = 0x1000 + ((ty * 32 + tx) * 2) as u32;
            vdp.write_vram(offset, (tile_entry & 0xFF) as u8);
            vdp.write_vram(offset + 1, ((tile_entry >> 8) & 0xFF) as u8);
        }
    }
    println!("  Created 32x32 tilemap with pattern");

    // Test 1: Identity transformation (affine mode)
    println!("\n=== Test 1: Identity Transformation ===");
    vdp.write_reg(
        VdpRegister::Bg0Control as u32,
        BgControl::ENABLE.bits() | BgControl::AFFINE.bits(),
    );

    // Identity matrix: scale 1.0, no rotation (8.8 fixed point)
    vdp.write_reg(VdpRegister::Bg0AffineA as u32, 0x0100); // pa = 1.0
    vdp.write_reg(VdpRegister::Bg0AffineB as u32, 0x0000); // pb = 0.0
    vdp.write_reg(VdpRegister::Bg0AffineC as u32, 0x0000); // pc = 0.0
    vdp.write_reg(VdpRegister::Bg0AffineD as u32, 0x0100); // pd = 1.0

    // Reference point at origin (8.8 fixed point)
    vdp.write_reg(VdpRegister::Bg0RefX as u32, 0x0000);
    vdp.write_reg(VdpRegister::Bg0RefX as u32 + 2, 0x0000);
    vdp.write_reg(VdpRegister::Bg0RefY as u32, 0x0000);
    vdp.write_reg(VdpRegister::Bg0RefY as u32 + 2, 0x0000);

    println!("  Matrix: [1.0, 0.0; 0.0, 1.0]");
    println!("  Reference: (0, 0)");

    // Render a frame
    let cycles_per_frame = Vdp::CYCLES_PER_SCANLINE * Vdp::SCANLINES_PER_FRAME as u64;
    vdp.step(cycles_per_frame);

    let fb = vdp.framebuffer();
    println!("  Rendered frame with {} pixels", fb.len());

    // Test 2: 2x scaling transformation
    println!("\n=== Test 2: 2x Scaling Transformation ===");

    // Scale by 2.0 in both directions (8.8 fixed point: 2.0 = 0x0200)
    vdp.write_reg(VdpRegister::Bg0AffineA as u32, 0x0200); // pa = 2.0
    vdp.write_reg(VdpRegister::Bg0AffineB as u32, 0x0000); // pb = 0.0
    vdp.write_reg(VdpRegister::Bg0AffineC as u32, 0x0000); // pc = 0.0
    vdp.write_reg(VdpRegister::Bg0AffineD as u32, 0x0200); // pd = 2.0

    println!("  Matrix: [2.0, 0.0; 0.0, 2.0]");
    println!("  Reference: (0, 0)");

    vdp.step(cycles_per_frame);
    println!("  Rendered scaled frame");

    // Test 3: 0.5x scaling (zoom out)
    println!("\n=== Test 3: 0.5x Scaling Transformation ===");

    // Scale by 0.5 in both directions (8.8 fixed point: 0.5 = 0x0080)
    vdp.write_reg(VdpRegister::Bg0AffineA as u32, 0x0080); // pa = 0.5
    vdp.write_reg(VdpRegister::Bg0AffineB as u32, 0x0000); // pb = 0.0
    vdp.write_reg(VdpRegister::Bg0AffineC as u32, 0x0000); // pc = 0.0
    vdp.write_reg(VdpRegister::Bg0AffineD as u32, 0x0080); // pd = 0.5

    println!("  Matrix: [0.5, 0.0; 0.0, 0.5]");
    println!("  Reference: (0, 0)");

    vdp.step(cycles_per_frame);
    println!("  Rendered zoomed-out frame");

    // Test 4: Non-affine mode (simple scrolling)
    println!("\n=== Test 4: Non-Affine Mode (Simple Scrolling) ===");

    // Disable affine mode
    vdp.write_reg(VdpRegister::Bg0Control as u32, BgControl::ENABLE.bits());

    // Set scroll values
    vdp.write_reg(VdpRegister::Bg0ScrollX as u32, 16);
    vdp.write_reg(VdpRegister::Bg0ScrollY as u32, 8);

    println!("  Affine mode: disabled");
    println!("  Scroll: (16, 8)");

    vdp.step(cycles_per_frame);
    println!("  Rendered scrolled frame");

    // Test 5: Rotation simulation (45 degrees approximation)
    println!("\n=== Test 5: Rotation Simulation (45° approx) ===");

    // Re-enable affine mode
    vdp.write_reg(
        VdpRegister::Bg0Control as u32,
        BgControl::ENABLE.bits() | BgControl::AFFINE.bits(),
    );

    // 45-degree rotation matrix (cos(45°) ≈ 0.707, sin(45°) ≈ 0.707)
    // In 8.8 fixed point: 0.707 ≈ 0x00B5 (181/256)
    // Matrix: [cos, -sin; sin, cos]
    vdp.write_reg(VdpRegister::Bg0AffineA as u32, 0x00B5); // pa = cos(45°)
    vdp.write_reg(VdpRegister::Bg0AffineB as u32, (-0xB5i16) as u16); // pb = -sin(45°)
    vdp.write_reg(VdpRegister::Bg0AffineC as u32, 0x00B5); // pc = sin(45°)
    vdp.write_reg(VdpRegister::Bg0AffineD as u32, 0x00B5); // pd = cos(45°)

    // Set reference point to center of screen (in 8.8 fixed point)
    let center_x = ((width / 2) as i32) << 8;
    let center_y = ((height / 2) as i32) << 8;
    vdp.write_reg(VdpRegister::Bg0RefX as u32, (center_x & 0xFFFF) as u16);
    vdp.write_reg(
        VdpRegister::Bg0RefX as u32 + 2,
        ((center_x >> 16) & 0xFF) as u16,
    );
    vdp.write_reg(VdpRegister::Bg0RefY as u32, (center_y & 0xFFFF) as u16);
    vdp.write_reg(
        VdpRegister::Bg0RefY as u32 + 2,
        ((center_y >> 16) & 0xFF) as u16,
    );

    println!("  Matrix: [0.707, -0.707; 0.707, 0.707] (45° rotation)");
    println!("  Reference: ({}, {})", width / 2, height / 2);

    vdp.step(cycles_per_frame);
    println!("  Rendered rotated frame");

    // Final status
    println!("\n=== VDP Status ===");
    println!("  Frame count: {}", vdp.frame_count());
    println!("  Current scanline: {}", vdp.scanline());
    println!("  In VBLANK: {}", vdp.in_vblank());

    println!("\n✓ BG0 affine transformation demo completed successfully!");
}
