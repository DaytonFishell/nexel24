//! VDP-T Demo
//!
//! Demonstrates basic VDP functionality:
//! - Display initialization
//! - Palette loading
//! - Sprite rendering
//! - VRAM/CRAM access

use nexel_core::vdp::{SpriteAttr, SpriteSize, Vdp};

fn main() {
    println!("Nexel-24 VDP-T Demo");
    println!("===================\n");

    // Create VDP instance
    let mut vdp = Vdp::new();

    // Configure display
    println!("Configuring display...");
    vdp.set_display_mode(320, 240);
    vdp.set_layer_enable(true, true, true);
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

    // Create simple 8x8 tile data (a filled square)
    println!("\nLoading tile data...");
    let mut tile_data = vec![0u8; 64]; // 8x8 pixels
    for y in 0..8 {
        for x in 0..8 {
            // Create a simple pattern
            let color = if x == 0 || x == 7 || y == 0 || y == 7 {
                7 // White border
            } else {
                1 // Red fill
            };
            tile_data[y * 8 + x] = color;
        }
    }
    vdp.load_tile_data(0, &tile_data);
    println!("  Loaded 8x8 tile with border pattern");

    // Configure sprites
    println!("\nConfiguring sprites...");

    // Sprite 0: Simple 8x8 sprite
    let sprite0 = SpriteAttr {
        y_pos: 100,
        x_pos: 100,
        tile_index: 0,
        attr: 0x8000, // Enabled, palette 0, 8x8 size
    };
    vdp.set_sprite(0, sprite0);
    println!("  Sprite 0: 8x8 at (100, 100)");

    // Sprite 1: 16x16 sprite
    let sprite1 = SpriteAttr {
        y_pos: 120,
        x_pos: 150,
        tile_index: 0,
        attr: 0x8101, // Enabled, palette 1, 16x16 size
    };
    vdp.set_sprite(1, sprite1);
    println!("  Sprite 1: 16x16 at (150, 120)");

    // Run VDP for a few frames
    println!("\nSimulating VDP timing...");
    for frame in 0..5 {
        // Simulate one frame worth of cycles
        let cycles_per_frame = Vdp::CYCLES_PER_SCANLINE * Vdp::SCANLINES_PER_FRAME as u64;
        let vblank = vdp.step(cycles_per_frame);

        if vblank {
            println!(
                "  Frame {}: VBLANK triggered, scanline={}",
                frame,
                vdp.scanline()
            );
        }
    }

    // Check VDP state
    println!("\nVDP Status:");
    println!("  Frame count: {}", vdp.frame_count());
    println!("  Current scanline: {}", vdp.scanline());
    println!("  In VBLANK: {}", vdp.in_vblank());
    println!("  In HBLANK: {}", vdp.in_hblank());

    // Verify VRAM access
    println!("\nVerifying VRAM access...");
    let read_tile = vdp.read_vram(0);
    println!("  First tile pixel color: {}", read_tile);

    // Verify CRAM access
    println!("\nVerifying CRAM access...");
    let r = vdp.read_cram(3); // Red color
    let g = vdp.read_cram(4);
    let b = vdp.read_cram(5);
    println!("  Color 1 (Red) RGB: ({}, {}, {})", r, g, b);

    // Get framebuffer info
    let fb = vdp.framebuffer();
    println!("\nFramebuffer info:");
    println!("  Size: {} pixels", fb.len());
    println!("  Dimensions: {}x{}", width, height);
    println!("  First pixel color: 0x{:06X}", fb[0]);

    println!("\n✓ VDP demo completed successfully!");
}
