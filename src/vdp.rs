// Copyright (C) 2025 Dayton Fishell
// Nexel-24 Game Console Emulator
// This file is part of Nexel-24.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version. See the LICENSE file in the project root for details.
// SPDX-License-Identifier: GPL-3.0-or-later

//! VDP-T (Tile/Sprite GPU) subsystem
//!
//! The VDP-T is the Nexel-24's graphics coprocessor, providing:
//! - 384x288 native resolution (with 320x240 and 256x224 modes)
//! - 512KB VRAM for tiles, sprites, and framebuffer
//! - 64KB CRAM for color palettes (18-bit color depth)
//! - 2 background layers (BG0 with affine, BG1 static tilemap)
//! - Up to 128 sprites on screen, 64 per scanline
//! - Hardware DMA with vblank/hblank triggers
//! - 4000 flat triangles/sec polygon rendering

use bitflags::bitflags;

/// VDP-T register offsets (memory-mapped at 0x100000-0x10FFFF)
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum VdpRegister {
    // Display control
    DisplayControl = 0x0000,
    DisplayStatus = 0x0002,
    VCount = 0x0004,
    HCount = 0x0006,

    // Background layers
    Bg0Control = 0x0010,
    Bg0ScrollX = 0x0012,
    Bg0ScrollY = 0x0014,
    Bg0AffineA = 0x0016,     // sx (scale x)
    Bg0AffineB = 0x0018,     // shx (shear x)
    Bg0AffineC = 0x001A,     // shy (shear y)
    Bg0AffineD = 0x001C,     // sy (scale y)
    Bg0RefX = 0x001E,        // reference point x (24-bit)
    Bg0RefY = 0x0022,        // reference point y (24-bit)
    Bg0TilemapAddr = 0x0026, // tilemap address

    Bg1Control = 0x0030,
    Bg1ScrollX = 0x0032,
    Bg1ScrollY = 0x0034,
    Bg1TilemapAddr = 0x0036,

    // Sprite control
    SpriteControl = 0x0050,
    SpriteOamAddr = 0x0052,

    // DMA control
    DmaSource = 0x0070,
    DmaDestination = 0x0074,
    DmaLength = 0x0078,
    DmaControl = 0x007A,

    // Interrupt control
    IrqEnable = 0x0080,
    IrqStatus = 0x0082,
    IrqLineCompare = 0x0084,

    // Color/palette
    PaletteIndex = 0x0090,
    PaletteData = 0x0092,
    BackdropColor = 0x0094,
}

bitflags! {
    /// Display control flags (DISPCTL register)
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct DisplayControl: u16 {
        const ENABLE = 1 << 0;          // Master display enable
        const BG0_ENABLE = 1 << 1;      // Enable BG0 layer
        const BG1_ENABLE = 1 << 2;      // Enable BG1 layer
        const SPRITE_ENABLE = 1 << 3;   // Enable sprites
        const POLYGON_ENABLE = 1 << 4;  // Enable polygon rendering
        const HBLANK_IRQ = 1 << 8;      // Enable HBLANK interrupt
        const VBLANK_IRQ = 1 << 9;      // Enable VBLANK interrupt
        const LINECMP_IRQ = 1 << 10;    // Enable line compare interrupt
        const MODE_320x240 = 1 << 12;   // Video mode: 320x240
        const MODE_256x224 = 1 << 13;   // Video mode: 256x224
    }
}

bitflags! {
    /// Display status flags (DISPSTAT register)
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct DisplayStatus: u16 {
        const VBLANK = 1 << 0;          // In VBLANK period
        const HBLANK = 1 << 1;          // In HBLANK period
        const LINECMP = 1 << 2;         // Line compare match
        const DMA_BUSY = 1 << 3;        // DMA in progress
        const CMDLIST_BUSY = 1 << 4;    // Command list processing
    }
}

bitflags! {
    /// Background control flags
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct BgControl: u16 {
        const ENABLE = 1 << 0;
        const PRIORITY_1 = 1 << 4;
        const PRIORITY_2 = 1 << 5;
        const MOSAIC = 1 << 6;
        const COLOR_256 = 1 << 7;       // 256-color mode (vs 16-color)
        const AFFINE = 1 << 8;          // Affine transformation mode
        const WRAPAROUND = 1 << 9;
        const SIZE_32x32 = 0 << 10;
        const SIZE_64x64 = 1 << 10;
        const SIZE_128x128 = 2 << 10;
    }
}

bitflags! {
    /// Sprite control flags (placeholder)
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SpriteControl: u16 {
        const ENABLE = 1 << 0; // Placeholder flag
        const SIZE_16 = 1 << 1; // Placeholder
    }
}

bitflags! {
    /// IRQ enable/status flags
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct IrqFlags: u16 {
        const HBLANK = 1 << 0;
        const VBLANK = 1 << 1;
        const LINECMP = 1 << 2;
        const DMA_DONE = 1 << 3;
    }
}

/// Sprite attribute entry (8 bytes in OAM)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpriteAttr {
    pub y_pos: u16,      // Y position (0-511)
    pub x_pos: u16,      // X position (0-511)
    pub tile_index: u16, // Tile index in VRAM
    pub attr: u16,       // Attributes (palette, flip, priority, size)
}

impl SpriteAttr {
    pub fn is_enabled(&self) -> bool {
        self.attr & 0x8000 != 0
    }

    pub fn palette(&self) -> u8 {
        ((self.attr >> 8) & 0xF) as u8
    }

    pub fn flip_h(&self) -> bool {
        self.attr & 0x1000 != 0
    }

    pub fn flip_v(&self) -> bool {
        self.attr & 0x2000 != 0
    }

    pub fn priority(&self) -> u8 {
        ((self.attr >> 10) & 0x3) as u8
    }

    pub fn size(&self) -> SpriteSize {
        match self.attr & 0x3 {
            0 => SpriteSize::Size8x8,
            1 => SpriteSize::Size16x16,
            2 => SpriteSize::Size32x32,
            3 => SpriteSize::Size64x64,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteSize {
    Size8x8,
    Size16x16,
    Size32x32,
    Size64x64,
}

impl SpriteSize {
    pub fn dimensions(&self) -> (u16, u16) {
        match self {
            SpriteSize::Size8x8 => (8, 8),
            SpriteSize::Size16x16 => (16, 16),
            SpriteSize::Size32x32 => (32, 32),
            SpriteSize::Size64x64 => (64, 64),
        }
    }
}

/// Main VDP-T state
pub struct Vdp {
    // Video RAM (512KB) - tiles, sprites, framebuffer
    vram: Vec<u8>,

    // Color RAM (64KB) - palette data (18-bit RGB666 colors)
    cram: Vec<u8>,

    // Register file
    regs: [u8; 256],

    // Display control
    display_control: DisplayControl,
    display_status: DisplayStatus,

    // Scanline counters
    v_count: u16, // Current scanline (0-287 for 288 lines)
    h_count: u16, // Current horizontal position

    // Background layers
    bg0_control: BgControl,
    bg0_scroll_x: i16,
    bg0_scroll_y: i16,
    bg0_affine: [i16; 4],  // A, B, C, D matrix parameters
    bg0_ref_x: i32,        // Reference point X (24-bit fixed point)
    bg0_ref_y: i32,        // Reference point Y (24-bit fixed point)
    bg0_tilemap_addr: u32, // Tilemap base address in VRAM

    bg1_control: BgControl,
    bg1_scroll_x: i16,
    bg1_scroll_y: i16,
    bg1_tilemap_addr: u32,

    // Sprite OAM (Object Attribute Memory) - 128 sprites * 8 bytes
    oam: Vec<SpriteAttr>,

    // Sprite and OAM control
    sprite_control: SpriteControl,
    sprite_oam_addr: u16,

    // DMA state
    dma_source: u32,
    dma_dest: u32,
    dma_length: u16,
    dma_active: bool,

    // IRQ registers
    irq_enable: IrqFlags,
    irq_status: IrqFlags,
    irq_line_compare: u16,

    // Palette registers
    palette_index: u8,
    palette_data: u8,

    // Backdrop color (cached 16-bit value)
    backdrop_color: u16,

    // Framebuffer for rendering (384x288, 18-bit color stored as u32)
    framebuffer: Vec<u32>,

    // Timing
    cycles: u64,
    frame_count: u64,
}

impl Vdp {
    pub const VRAM_SIZE: usize = 0x80000; // 512KB
    pub const CRAM_SIZE: usize = 0x10000; // 64KB
    pub const OAM_SPRITES: usize = 128;

    pub const NATIVE_WIDTH: usize = 384;
    pub const NATIVE_HEIGHT: usize = 288;
    pub const MODE_320_WIDTH: usize = 320;
    pub const MODE_320_HEIGHT: usize = 240;
    pub const MODE_256_WIDTH: usize = 256;
    pub const MODE_256_HEIGHT: usize = 224;

    // Timing constants (for 18.432 MHz system clock)
    pub const CYCLES_PER_SCANLINE: u64 = 1024;
    pub const SCANLINES_PER_FRAME: u16 = 288;
    pub const VBLANK_START: u16 = 240; // Start of VBLANK

    pub fn new() -> Self {
        Self {
            vram: vec![0; Self::VRAM_SIZE],
            cram: vec![0; Self::CRAM_SIZE],
            regs: [0; 256],
            display_control: DisplayControl::empty(),
            display_status: DisplayStatus::empty(),
            v_count: 0,
            h_count: 0,
            bg0_control: BgControl::empty(),
            bg0_scroll_x: 0,
            bg0_scroll_y: 0,
            bg0_affine: [0x100, 0, 0, 0x100], // Identity matrix (1.0 scale)
            bg0_ref_x: 0,
            bg0_ref_y: 0,
            bg0_tilemap_addr: 0,
            bg1_control: BgControl::empty(),
            bg1_scroll_x: 0,
            bg1_scroll_y: 0,
            bg1_tilemap_addr: 0,
            oam: vec![
                SpriteAttr {
                    y_pos: 0,
                    x_pos: 0,
                    tile_index: 0,
                    attr: 0,
                };
                Self::OAM_SPRITES
            ],
            sprite_control: SpriteControl::empty(),
            sprite_oam_addr: 0,
            dma_source: 0,
            dma_dest: 0,
            dma_length: 0,
            dma_active: false,
            irq_enable: IrqFlags::empty(),
            irq_status: IrqFlags::empty(),
            irq_line_compare: 0,
            palette_index: 0,
            palette_data: 0,
            backdrop_color: 0,
            framebuffer: vec![0; Self::NATIVE_WIDTH * Self::NATIVE_HEIGHT],
            cycles: 0,
            frame_count: 0,
        }
    }

    /// Advance VDP timing by the specified number of cycles
    pub fn step(&mut self, cycles: u64) -> bool {
        self.cycles += cycles;

        // Update scanline position
        let old_v = self.v_count;
        let scanline_cycles = self.cycles / Self::CYCLES_PER_SCANLINE;
        self.v_count = (scanline_cycles % Self::SCANLINES_PER_FRAME as u64) as u16;
        self.h_count = (self.cycles % Self::CYCLES_PER_SCANLINE) as u16;

        // Update display status flags
        self.display_status
            .set(DisplayStatus::VBLANK, self.v_count >= Self::VBLANK_START);
        self.display_status
            .set(DisplayStatus::HBLANK, self.h_count >= 768);

        // Check for VBLANK transition
        let entered_vblank = old_v < Self::VBLANK_START && self.v_count >= Self::VBLANK_START;

        if entered_vblank {
            self.frame_count += 1;
            if self.display_control.contains(DisplayControl::ENABLE) {
                self.render_frame();
            }
        }

        entered_vblank
    }

    /// Read a 16-bit register
    pub fn read_reg(&self, offset: u32) -> u16 {
        match offset {
            0x0000 => self.display_control.bits(),
            0x0002 => self.display_status.bits(),
            0x0004 => self.v_count,
            0x0006 => self.h_count,
            0x0010 => self.bg0_control.bits(),
            0x0012 => self.bg0_scroll_x as u16,
            0x0014 => self.bg0_scroll_y as u16,
            0x0016 => self.bg0_affine[0] as u16,
            0x0018 => self.bg0_affine[1] as u16,
            0x001A => self.bg0_affine[2] as u16,
            0x001C => self.bg0_affine[3] as u16,
            0x001E => (self.bg0_ref_x & 0xFFFF) as u16,
            0x0020 => ((self.bg0_ref_x >> 16) & 0xFF) as u16,
            0x0022 => (self.bg0_ref_y & 0xFFFF) as u16,
            0x0024 => ((self.bg0_ref_y >> 16) & 0xFF) as u16,
            0x0026 => self.bg0_tilemap_addr as u16,
            0x0030 => self.bg1_control.bits(),
            0x0032 => self.bg1_scroll_x as u16,
            0x0034 => self.bg1_scroll_y as u16,
            0x0036 => self.bg1_tilemap_addr as u16,
            0x0050 => self.sprite_control.bits(),
            0x0052 => self.sprite_oam_addr as u16,
            0x0070 => (self.dma_source & 0xFFFF) as u16,
            0x0072 => ((self.dma_source >> 16) & 0xFF) as u16,
            0x0074 => (self.dma_dest & 0xFFFF) as u16,
            0x0076 => ((self.dma_dest >> 16) & 0xFF) as u16,
            0x0078 => self.dma_length,
            0x007A => {
                // DMA control - read as 0 (not used)
                0
            }
            0x0080 => self.irq_enable.bits(),
            0x0082 => self.irq_status.bits(),
            0x0084 => self.irq_line_compare,
            0x0090 => self.palette_index as u16,
            0x0092 => self.palette_data as u16,
            0x0094 => self.backdrop_color as u16,
            _ => {
                // Default to reading from raw register array
                let idx = (offset as usize) % self.regs.len();
                if idx + 1 < self.regs.len() {
                    u16::from_le_bytes([self.regs[idx], self.regs[idx + 1]])
                } else {
                    0xFF
                }
            }
        }
    }

    /// Write a 16-bit register
    pub fn write_reg(&mut self, offset: u32, value: u16) {
        match offset {
            0x0000 => {
                self.display_control = DisplayControl::from_bits_truncate(value);
            }
            0x0004 => {} // VCount is read-only
            0x0006 => {} // HCount is read-only
            0x0010 => {
                self.bg0_control = BgControl::from_bits_truncate(value);
            }
            0x0012 => self.bg0_scroll_x = value as i16,
            0x0014 => self.bg0_scroll_y = value as i16,
            0x0016 => self.bg0_affine[0] = value as i16,
            0x0018 => self.bg0_affine[1] = value as i16,
            0x001A => self.bg0_affine[2] = value as i16,
            0x001C => self.bg0_affine[3] = value as i16,
            0x001E => {
                // RefX low word
                self.bg0_ref_x = (self.bg0_ref_x & !0xFFFF) | (value as i32);
            }
            0x0020 => {
                // RefX high byte (24-bit address)
                self.bg0_ref_x = (self.bg0_ref_x & 0x0000FFFF) | (((value as i32) & 0xFF) << 16);
            }
            0x0022 => {
                // RefY low word
                self.bg0_ref_y = (self.bg0_ref_y & !0xFFFF) | (value as i32);
            }
            0x0024 => {
                // RefY high byte (24-bit address)
                self.bg0_ref_y = (self.bg0_ref_y & 0x0000FFFF) | (((value as i32) & 0xFF) << 16);
            }
            0x0026 => self.bg0_tilemap_addr = value as u32,
            0x0030 => {
                self.bg1_control = BgControl::from_bits_truncate(value);
            }
            0x0032 => self.bg1_scroll_x = value as i16,
            0x0034 => self.bg1_scroll_y = value as i16,
            0x0036 => self.bg1_tilemap_addr = value as u32,
            0x0050 => {
                self.sprite_control = SpriteControl::from_bits_truncate(value);
            }
            0x0052 => {
                // Sprite OAM base address
                self.sprite_oam_addr = value as u16;
            }
            0x0070 => {
                // DMA source low word
                self.dma_source = (self.dma_source & 0xFFFF0000) | value as u32;
            }
            0x0072 => {
                // DMA source high byte (24-bit address)
                self.dma_source = (self.dma_source & 0x0000FFFF) | ((value as u32 & 0xFF) << 16);
            }
            0x0074 => {
                // DMA destination low word
                self.dma_dest = (self.dma_dest & 0xFFFF0000) | value as u32;
            }
            0x0076 => {
                // DMA destination high byte
                self.dma_dest = (self.dma_dest & 0x0000FFFF) | ((value as u32 & 0xFF) << 16);
            }
            0x0078 => self.dma_length = value,
            0x007A => {
                // DMA control - writing initiates transfer
                if value & 0x8000 != 0 {
                    self.start_dma();
                }
            }
            0x0080 => {
                self.irq_enable = IrqFlags::from_bits_truncate(value);
            }
            0x0082 => {
                // Writing to IRQ status clears bits specified
                let mask = IrqFlags::from_bits_truncate(value);
                self.irq_status.remove(mask);
            }
            0x0084 => {
                self.irq_line_compare = value;
            }
            0x0090 => {
                self.palette_index = value as u8;
            }
            0x0092 => {
                self.palette_data = value as u8;
                // Writing palette data stores into CRAM at current index
                let idx = (self.palette_index as u32) * 3;
                // For simplicity, write the low byte of value into the palette data
                self.write_cram(idx, (self.palette_data & 0x3F) as u8);
            }
            0x0094 => {
                self.backdrop_color = value;
                // Also update CRAM[0..3] with 18-bit color
                let r = (value & 0x3F) as u8;
                let g = ((value >> 6) & 0x3F) as u8;
                let b = ((value >> 12) & 0x3F) as u8;
                self.set_backdrop_color(r, g, b);
            }
            _ => {
                // Write to raw register array
                let idx = (offset as usize) % self.regs.len();
                if idx + 1 < self.regs.len() {
                    let bytes = value.to_le_bytes();
                    self.regs[idx] = bytes[0];
                    self.regs[idx + 1] = bytes[1];
                }
            }
        }
    }

    /// Read a byte from VRAM
    pub fn read_vram(&self, offset: u32) -> u8 {
        self.vram
            .get((offset as usize) % Self::VRAM_SIZE)
            .copied()
            .unwrap_or(0xFF)
    }

    /// Write a byte to VRAM
    pub fn write_vram(&mut self, offset: u32, value: u8) {
        let idx = (offset as usize) % Self::VRAM_SIZE;
        if let Some(cell) = self.vram.get_mut(idx) {
            *cell = value;
        }
    }

    /// Read a byte from CRAM (color palette RAM)
    pub fn read_cram(&self, offset: u32) -> u8 {
        self.cram
            .get((offset as usize) % Self::CRAM_SIZE)
            .copied()
            .unwrap_or(0xFF)
    }

    /// Write a byte to CRAM
    pub fn write_cram(&mut self, offset: u32, value: u8) {
        let idx = (offset as usize) % Self::CRAM_SIZE;
        if let Some(cell) = self.cram.get_mut(idx) {
            *cell = value;
        }
    }

    /// Start a DMA transfer
    fn start_dma(&mut self) {
        self.dma_active = true;
        self.display_status.insert(DisplayStatus::DMA_BUSY);
        // TODO: Implement actual DMA transfer logic
        // For now, just mark as complete immediately
        self.dma_active = false;
        self.display_status.remove(DisplayStatus::DMA_BUSY);
    }

    /// Render the current frame to the framebuffer
    fn render_frame(&mut self) {
        // Clear framebuffer to backdrop color
        let backdrop = self.read_backdrop_color();
        for pixel in self.framebuffer.iter_mut() {
            *pixel = backdrop;
        }

        // Render layers in priority order
        if self.display_control.contains(DisplayControl::BG1_ENABLE) {
            self.render_bg1();
        }

        if self.display_control.contains(DisplayControl::BG0_ENABLE) {
            self.render_bg0();
        }

        if self.display_control.contains(DisplayControl::SPRITE_ENABLE) {
            self.render_sprites();
        }
    }

    /// Read the backdrop (background) color from CRAM
    fn read_backdrop_color(&self) -> u32 {
        // Backdrop color stored at CRAM offset 0 (18-bit RGB666)
        let r = self.cram[0];
        let g = self.cram[1];
        let b = self.cram[2];
        self.rgb666_to_rgb888(r, g, b)
    }

    /// Convert RGB666 (18-bit) to RGB888 (24-bit) for framebuffer
    fn rgb666_to_rgb888(&self, r: u8, g: u8, b: u8) -> u32 {
        let r8 = ((r & 0x3F) << 2) | ((r & 0x3F) >> 4);
        let g8 = ((g & 0x3F) << 2) | ((g & 0x3F) >> 4);
        let b8 = ((b & 0x3F) << 2) | ((b & 0x3F) >> 4);
        ((r8 as u32) << 16) | ((g8 as u32) << 8) | (b8 as u32)
    }

    /// Render BG0 layer (affine-capable background)
    fn render_bg0(&mut self) {
        if !self.bg0_control.contains(BgControl::ENABLE) {
            return;
        }

        let (width, height) = self.display_dimensions();

        // Determine tilemap size based on control flags
        let tile_map_size = if self.bg0_control.contains(BgControl::SIZE_128x128) {
            128
        } else if self.bg0_control.contains(BgControl::SIZE_64x64) {
            64
        } else {
            32
        };

        // Check if affine transformation is enabled
        if self.bg0_control.contains(BgControl::AFFINE) {
            // Affine mode: apply 2D transformation
            // Matrix parameters are in 8.8 fixed point format
            let pa = self.bg0_affine[0] as i32; // A (dx/dx)
            let pb = self.bg0_affine[1] as i32; // B (dx/dy)
            let pc = self.bg0_affine[2] as i32; // C (dy/dx)
            let pd = self.bg0_affine[3] as i32; // D (dy/dy)

            // Reference points store the texture coordinate (in 8.8 fixed point)
            // that should appear at the screen center
            let ref_x = self.bg0_ref_x;
            let ref_y = self.bg0_ref_y;

            let wraparound = self.bg0_control.contains(BgControl::WRAPAROUND);

            // Screen center coordinates
            let center_x = (width / 2) as i32;
            let center_y = (height / 2) as i32;

            // For each screen pixel, apply affine transformation
            for screen_y in 0..height {
                for screen_x in 0..width {
                    // Calculate offset from screen center
                    let dx = screen_x as i32 - center_x;
                    let dy = screen_y as i32 - center_y;

                    // Apply transformation matrix (8.8 fixed point math)
                    // Formula: [tex_x, tex_y] = [ref_x, ref_y] + Matrix * [dx, dy]
                    let tex_x = ref_x + ((pa * dx + pb * dy) >> 8);
                    let tex_y = ref_y + ((pc * dx + pd * dy) >> 8);

                    // Convert from 8.8 fixed point to integer pixel coordinates
                    let mut pixel_x = (tex_x >> 8) as i32;
                    let mut pixel_y = (tex_y >> 8) as i32;

                    // Handle wraparound or clipping
                    if wraparound {
                        let map_size = (tile_map_size * 8) as i32;
                        pixel_x = pixel_x.rem_euclid(map_size);
                        pixel_y = pixel_y.rem_euclid(map_size);
                    } else {
                        // Clip to tilemap bounds
                        if pixel_x < 0
                            || pixel_x >= (tile_map_size * 8) as i32
                            || pixel_y < 0
                            || pixel_y >= (tile_map_size * 8) as i32
                        {
                            continue; // Out of bounds, skip pixel
                        }
                    }

                    // Calculate tile coordinates
                    let tile_x = (pixel_x / 8) as u16;
                    let tile_y = (pixel_y / 8) as u16;
                    let px = (pixel_x % 8) as u16;
                    let py = (pixel_y % 8) as u16;

                    // Read tile index from tilemap
                    let tile_map_offset = ((tile_y * tile_map_size as u16 + tile_x) * 2) as u32;
                    let tilemap_offset = self.bg0_tilemap_addr + tile_map_offset;
                    let tile_entry = self.read_vram(tilemap_offset) as u16
                        | ((self.read_vram(tilemap_offset + 1) as u16) << 8);

                    let tile_index = tile_entry & 0x3FF; // 10-bit tile index
                    let palette = ((tile_entry >> 12) & 0xF) as u8;

                    // Note: In affine mode, flip flags are typically ignored
                    // Read pixel from tile data (8x8 tiles, 8 bits per pixel)
                    let tile_data_offset = (tile_index as u32 * 64) + (py as u32 * 8) + px as u32;
                    let color_index = self.read_vram(tile_data_offset);

                    // Skip transparent pixels (color 0)
                    if color_index == 0 {
                        continue;
                    }

                    // Read color from palette
                    let palette_offset = (palette as u32 * 256 * 3) + (color_index as u32 * 3);
                    let r = self.read_cram(palette_offset);
                    let g = self.read_cram(palette_offset + 1);
                    let b = self.read_cram(palette_offset + 2);

                    let color = self.rgb666_to_rgb888(r, g, b);

                    // Write to framebuffer
                    let fb_offset = screen_y * width + screen_x;
                    if let Some(pixel) = self.framebuffer.get_mut(fb_offset) {
                        *pixel = color;
                    }
                }
            }
        } else {
            // Non-affine mode: simple scrolling like BG1
            let scroll_x = self.bg0_scroll_x;
            let scroll_y = self.bg0_scroll_y;

            for screen_y in 0..height {
                for screen_x in 0..width {
                    // Apply scrolling
                    let world_x = (screen_x as i16).wrapping_add(scroll_x) as u16;
                    let world_y = (screen_y as i16).wrapping_add(scroll_y) as u16;

                    // Calculate tile coordinates
                    let tile_x = (world_x / 8) % tile_map_size as u16;
                    let tile_y = (world_y / 8) % tile_map_size as u16;
                    let pixel_x = world_x % 8;
                    let pixel_y = world_y % 8;

                    // Read tile index from tilemap
                    let tile_map_offset = ((tile_y * tile_map_size as u16 + tile_x) * 2) as u32;
                    let tilemap_offset = self.bg0_tilemap_addr + tile_map_offset;
                    let tile_entry = self.read_vram(tilemap_offset) as u16
                        | ((self.read_vram(tilemap_offset + 1) as u16) << 8);

                    let tile_index = tile_entry & 0x3FF; // 10-bit tile index
                    let palette = ((tile_entry >> 12) & 0xF) as u8;
                    let flip_h = (tile_entry & 0x0400) != 0;
                    let flip_v = (tile_entry & 0x0800) != 0;

                    // Apply flipping
                    let px = if flip_h { 7 - pixel_x } else { pixel_x };
                    let py = if flip_v { 7 - pixel_y } else { pixel_y };

                    // Read pixel from tile data (8x8 tiles, 8 bits per pixel)
                    let tile_data_offset = (tile_index as u32 * 64) + (py as u32 * 8) + px as u32;
                    let color_index = self.read_vram(tile_data_offset);

                    // Skip transparent pixels (color 0)
                    if color_index == 0 {
                        continue;
                    }

                    // Read color from palette
                    let palette_offset = (palette as u32 * 256 * 3) + (color_index as u32 * 3);
                    let r = self.read_cram(palette_offset);
                    let g = self.read_cram(palette_offset + 1);
                    let b = self.read_cram(palette_offset + 2);

                    let color = self.rgb666_to_rgb888(r, g, b);

                    // Write to framebuffer
                    let fb_offset = screen_y * width + screen_x;
                    if let Some(pixel) = self.framebuffer.get_mut(fb_offset) {
                        *pixel = color;
                    }
                }
            }
        }
    }

    /// Render BG1 layer (static tilemap background)
    fn render_bg1(&mut self) {
        if !self.bg1_control.contains(BgControl::ENABLE) {
            return;
        }

        let (width, height) = self.display_dimensions();
        let scroll_x = self.bg1_scroll_x;
        let scroll_y = self.bg1_scroll_y;

        // Determine tilemap size based on control flags
        let tile_map_width = if self.bg1_control.contains(BgControl::SIZE_128x128) {
            128
        } else if self.bg1_control.contains(BgControl::SIZE_64x64) {
            64
        } else {
            32
        };

        let tile_map_height = tile_map_width; // Square tilemaps for now

        // Render each visible tile
        for screen_y in 0..height {
            for screen_x in 0..width {
                // Apply scrolling
                let world_x = (screen_x as i16).wrapping_add(scroll_x) as u16;
                let world_y = (screen_y as i16).wrapping_add(scroll_y) as u16;

                // Calculate tile coordinates
                let tile_x = (world_x / 8) % tile_map_width as u16;
                let tile_y = (world_y / 8) % tile_map_height as u16;
                let pixel_x = world_x % 8;
                let pixel_y = world_y % 8;

                // Read tile index from tilemap
                let tile_map_offset = ((tile_y * tile_map_width as u16 + tile_x) * 2) as u32;
                let tilemap_offset = self.bg1_tilemap_addr + tile_map_offset;
                let tile_entry = self.read_vram(tilemap_offset) as u16
                    | ((self.read_vram(tilemap_offset + 1) as u16) << 8);

                let tile_index = tile_entry & 0x3FF; // 10-bit tile index
                let palette = ((tile_entry >> 12) & 0xF) as u8;
                let flip_h = (tile_entry & 0x0400) != 0;
                let flip_v = (tile_entry & 0x0800) != 0;

                // Apply flipping
                let px = if flip_h { 7 - pixel_x } else { pixel_x };
                let py = if flip_v { 7 - pixel_y } else { pixel_y };

                // Read pixel from tile data (8x8 tiles, 8 bits per pixel for 256-color mode)
                let tile_data_offset = (tile_index * 64 + py * 8 + px) as u32;
                let color_index = self.read_vram(tile_data_offset);

                // Skip transparent pixels (color 0)
                if color_index == 0 {
                    continue;
                }

                // Read color from palette
                let palette_offset = (palette as u32 * 256 * 3) + (color_index as u32 * 3);
                let r = self.read_cram(palette_offset);
                let g = self.read_cram(palette_offset + 1);
                let b = self.read_cram(palette_offset + 2);

                let color = self.rgb666_to_rgb888(r, g, b);

                // Write to framebuffer
                let fb_offset = screen_y * width + screen_x;
                if let Some(pixel) = self.framebuffer.get_mut(fb_offset) {
                    *pixel = color;
                }
            }
        }
    }

    /// Render all active sprites
    fn render_sprites(&mut self) {
        let (width, height) = self.display_dimensions();

        // Sort sprites by priority (lower priority values render first, higher values on top)
        let mut sorted_sprites: Vec<(usize, &SpriteAttr)> = self
            .oam
            .iter()
            .enumerate()
            .filter(|(_, sprite)| sprite.is_enabled())
            .collect();

        sorted_sprites.sort_by_key(|(_, sprite)| sprite.priority());

        // Track sprites per scanline for hardware limit (64 per scanline)
        let mut scanline_sprite_counts = vec![0u8; height];

        for (_, sprite) in sorted_sprites.iter() {
            let (sprite_width, sprite_height) = sprite.size().dimensions();

            // Check if sprite is visible
            if sprite.x_pos >= width as u16 && sprite.x_pos < 512 {
                continue; // Off-screen right
            }
            if sprite.y_pos >= height as u16 && sprite.y_pos < 512 {
                continue; // Off-screen bottom
            }

            // Check scanline limit
            let y_start = sprite.y_pos.min(height as u16) as usize;
            let y_end = (sprite.y_pos + sprite_height).min(height as u16) as usize;

            let mut scanline_limited = false;
            for y in y_start..y_end {
                if scanline_sprite_counts[y] >= 64 {
                    scanline_limited = true;
                    break;
                }
            }

            if scanline_limited {
                continue; // Skip this sprite due to scanline limit
            }

            // Render sprite pixels
            for sprite_y in 0..sprite_height {
                let screen_y = sprite.y_pos.wrapping_add(sprite_y) as usize;
                if screen_y >= height {
                    continue;
                }

                // Increment scanline sprite count
                if scanline_sprite_counts[screen_y] < 64 {
                    scanline_sprite_counts[screen_y] += 1;
                }

                for sprite_x in 0..sprite_width {
                    let screen_x = sprite.x_pos.wrapping_add(sprite_x) as usize;
                    if screen_x >= width {
                        continue;
                    }

                    // Apply flipping
                    let px = if sprite.flip_h() {
                        sprite_width - 1 - sprite_x
                    } else {
                        sprite_x
                    };
                    let py = if sprite.flip_v() {
                        sprite_height - 1 - sprite_y
                    } else {
                        sprite_y
                    };

                    // Read pixel from sprite tile data
                    // Sprite tiles are stored as 8x8 tiles, arranged in sprite_width/8 x sprite_height/8 grid
                    let tile_x = px / 8;
                    let tile_y = py / 8;
                    let pixel_x = px % 8;
                    let pixel_y = py % 8;

                    let tiles_per_row = sprite_width / 8;
                    let tile_offset = tile_y * tiles_per_row + tile_x;
                    let tile_index = sprite.tile_index + tile_offset;

                    // Read pixel from tile data (8 bits per pixel)
                    let tile_data_offset =
                        (tile_index as u32 * 64) + (pixel_y as u32 * 8) + pixel_x as u32;
                    let color_index = self.read_vram(tile_data_offset);

                    // Skip transparent pixels (color 0)
                    if color_index == 0 {
                        continue;
                    }

                    // Read color from sprite palette
                    let palette_offset =
                        (sprite.palette() as u32 * 256 * 3) + (color_index as u32 * 3);
                    let r = self.read_cram(palette_offset);
                    let g = self.read_cram(palette_offset + 1);
                    let b = self.read_cram(palette_offset + 2);

                    let color = self.rgb666_to_rgb888(r, g, b);

                    // Write to framebuffer
                    let fb_offset = screen_y * width + screen_x;
                    if let Some(pixel) = self.framebuffer.get_mut(fb_offset) {
                        *pixel = color;
                    }
                }
            }
        }
    }

    /// Get a reference to the framebuffer
    pub fn framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    /// Get current display dimensions based on mode
    pub fn display_dimensions(&self) -> (usize, usize) {
        if self.display_control.contains(DisplayControl::MODE_320x240) {
            (Self::MODE_320_WIDTH, Self::MODE_320_HEIGHT)
        } else if self.display_control.contains(DisplayControl::MODE_256x224) {
            (Self::MODE_256_WIDTH, Self::MODE_256_HEIGHT)
        } else {
            (Self::NATIVE_WIDTH, Self::NATIVE_HEIGHT)
        }
    }

    /// Check if currently in VBLANK period
    pub fn in_vblank(&self) -> bool {
        self.display_status.contains(DisplayStatus::VBLANK)
    }

    /// Check if currently in HBLANK period
    pub fn in_hblank(&self) -> bool {
        self.display_status.contains(DisplayStatus::HBLANK)
    }

    /// Get current scanline
    pub fn scanline(&self) -> u16 {
        self.v_count
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Set display mode
    pub fn set_display_mode(&mut self, width: usize, height: usize) {
        self.display_control
            .remove(DisplayControl::MODE_320x240 | DisplayControl::MODE_256x224);

        match (width, height) {
            (320, 240) => self.display_control.insert(DisplayControl::MODE_320x240),
            (256, 224) => self.display_control.insert(DisplayControl::MODE_256x224),
            _ => {} // Default to native 384x288
        }
    }

    /// Enable or disable display layers
    pub fn set_layer_enable(&mut self, bg0: bool, bg1: bool, sprites: bool) {
        self.display_control.set(DisplayControl::BG0_ENABLE, bg0);
        self.display_control.set(DisplayControl::BG1_ENABLE, bg1);
        self.display_control
            .set(DisplayControl::SPRITE_ENABLE, sprites);
    }

    /// Enable master display
    pub fn set_display_enable(&mut self, enable: bool) {
        self.display_control.set(DisplayControl::ENABLE, enable);
    }

    /// Get OAM entry by index
    pub fn get_sprite(&self, index: usize) -> Option<&SpriteAttr> {
        self.oam.get(index)
    }

    /// Set OAM entry by index
    pub fn set_sprite(&mut self, index: usize, sprite: SpriteAttr) {
        if let Some(entry) = self.oam.get_mut(index) {
            *entry = sprite;
        }
    }

    /// Load tile data into VRAM
    pub fn load_tile_data(&mut self, offset: u32, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.write_vram(offset + i as u32, byte);
        }
    }

    /// Load palette data into CRAM
    pub fn load_palette(&mut self, palette_index: u8, colors: &[(u8, u8, u8)]) {
        let offset = palette_index as u32 * 256 * 3;
        for (i, &(r, g, b)) in colors.iter().enumerate() {
            let color_offset = offset + (i as u32 * 3);
            self.write_cram(color_offset, r & 0x3F); // 6-bit red
            self.write_cram(color_offset + 1, g & 0x3F); // 6-bit green
            self.write_cram(color_offset + 2, b & 0x3F); // 6-bit blue
        }
    }

    /// Set backdrop color
    pub fn set_backdrop_color(&mut self, r: u8, g: u8, b: u8) {
        self.cram[0] = r & 0x3F;
        self.cram[1] = g & 0x3F;
        self.cram[2] = b & 0x3F;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdp_initialization() {
        let vdp = Vdp::new();
        assert_eq!(vdp.v_count, 0);
        assert_eq!(vdp.h_count, 0);
        assert_eq!(vdp.frame_count, 0);
        assert!(!vdp.display_control.contains(DisplayControl::ENABLE));
    }

    #[test]
    fn vdp_display_modes() {
        let mut vdp = Vdp::new();

        // Test native mode
        assert_eq!(vdp.display_dimensions(), (384, 288));

        // Test 320x240 mode
        vdp.set_display_mode(320, 240);
        assert_eq!(vdp.display_dimensions(), (320, 240));
        assert!(vdp.display_control.contains(DisplayControl::MODE_320x240));

        // Test 256x224 mode
        vdp.set_display_mode(256, 224);
        assert_eq!(vdp.display_dimensions(), (256, 224));
        assert!(vdp.display_control.contains(DisplayControl::MODE_256x224));
    }

    #[test]
    fn vdp_vram_access() {
        let mut vdp = Vdp::new();

        // Write and read VRAM
        vdp.write_vram(0x1000, 0x42);
        assert_eq!(vdp.read_vram(0x1000), 0x42);

        // Test wrapping
        vdp.write_vram(Vdp::VRAM_SIZE as u32, 0x55);
        assert_eq!(vdp.read_vram(0), 0x55);
    }

    #[test]
    fn vdp_cram_access() {
        let mut vdp = Vdp::new();

        // Write and read CRAM
        vdp.write_cram(0, 0x3F);
        vdp.write_cram(1, 0x20);
        vdp.write_cram(2, 0x10);

        assert_eq!(vdp.read_cram(0), 0x3F);
        assert_eq!(vdp.read_cram(1), 0x20);
        assert_eq!(vdp.read_cram(2), 0x10);
    }

    #[test]
    fn vdp_register_access() {
        let mut vdp = Vdp::new();

        // Write display control
        vdp.write_reg(VdpRegister::DisplayControl as u32, 0x0007);
        assert_eq!(vdp.read_reg(VdpRegister::DisplayControl as u32), 0x0007);
        assert!(vdp.display_control.contains(DisplayControl::ENABLE));
        assert!(vdp.display_control.contains(DisplayControl::BG0_ENABLE));
        assert!(vdp.display_control.contains(DisplayControl::BG1_ENABLE));
    }

    #[test]
    fn vdp_timing() {
        let mut vdp = Vdp::new();

        // Step one scanline
        let vblank = vdp.step(Vdp::CYCLES_PER_SCANLINE);
        assert!(!vblank);
        assert_eq!(vdp.v_count, 1);

        // Step to VBLANK
        let cycles_to_vblank = (Vdp::VBLANK_START as u64 - 1) * Vdp::CYCLES_PER_SCANLINE;
        let vblank = vdp.step(cycles_to_vblank);
        assert!(vblank);
        assert!(vdp.in_vblank());
        assert_eq!(vdp.frame_count, 1);
    }

    #[test]
    fn vdp_palette_loading() {
        let mut vdp = Vdp::new();

        let colors = vec![
            (0x00, 0x00, 0x00), // Black
            (0x3F, 0x00, 0x00), // Red
            (0x00, 0x3F, 0x00), // Green
            (0x00, 0x00, 0x3F), // Blue
        ];

        vdp.load_palette(0, &colors);

        // Check first color (black)
        assert_eq!(vdp.read_cram(0), 0x00);
        assert_eq!(vdp.read_cram(1), 0x00);
        assert_eq!(vdp.read_cram(2), 0x00);

        // Check red
        assert_eq!(vdp.read_cram(3), 0x3F);
        assert_eq!(vdp.read_cram(4), 0x00);
        assert_eq!(vdp.read_cram(5), 0x00);
    }

    #[test]
    fn vdp_sprite_attributes() {
        // Attribute bits: [15: enable] [14-13: flip] [12-10: priority] [11-8: palette] [1-0: size]
        // 0x8101: enabled (bit 15), palette 1 (bits 11-8), priority 0, size 1 (16x16)
        let sprite = SpriteAttr {
            y_pos: 100,
            x_pos: 150,
            tile_index: 42,
            attr: 0x8101, // Enabled, palette 1, priority 0, 16x16 size
        };

        assert!(sprite.is_enabled());
        assert_eq!(sprite.palette(), 1);
        assert_eq!(sprite.priority(), 0);
        assert_eq!(sprite.size(), SpriteSize::Size16x16);
        assert_eq!(sprite.size().dimensions(), (16, 16));

        // Test different sizes
        let sprite_8x8 = SpriteAttr {
            y_pos: 0,
            x_pos: 0,
            tile_index: 0,
            attr: 0x8000, // Enabled, size 0 (8x8)
        };
        assert_eq!(sprite_8x8.size(), SpriteSize::Size8x8);

        let sprite_32x32 = SpriteAttr {
            y_pos: 0,
            x_pos: 0,
            tile_index: 0,
            attr: 0x8002, // Enabled, size 2 (32x32)
        };
        assert_eq!(sprite_32x32.size(), SpriteSize::Size32x32);
    }

    #[test]
    fn vdp_bg0_affine_registers() {
        let mut vdp = Vdp::new();

        // Test affine matrix registers
        vdp.write_reg(VdpRegister::Bg0AffineA as u32, 0x0200); // 2.0 scale
        vdp.write_reg(VdpRegister::Bg0AffineB as u32, 0x0080); // shear
        vdp.write_reg(VdpRegister::Bg0AffineC as u32, 0x0040); // shear
        vdp.write_reg(VdpRegister::Bg0AffineD as u32, 0x0180); // 1.5 scale

        assert_eq!(vdp.read_reg(VdpRegister::Bg0AffineA as u32), 0x0200);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0AffineB as u32), 0x0080);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0AffineC as u32), 0x0040);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0AffineD as u32), 0x0180);

        // Test that values are stored as i16
        assert_eq!(vdp.bg0_affine[0], 0x0200);
        assert_eq!(vdp.bg0_affine[1], 0x0080);
        assert_eq!(vdp.bg0_affine[2], 0x0040);
        assert_eq!(vdp.bg0_affine[3], 0x0180);
    }

    #[test]
    fn vdp_bg0_reference_point() {
        let mut vdp = Vdp::new();

        // Test RefX (24-bit register accessed as two 16-bit writes)
        vdp.write_reg(VdpRegister::Bg0RefX as u32, 0x1234); // Low word
        vdp.write_reg(VdpRegister::Bg0RefX as u32 + 2, 0x0056); // High byte

        assert_eq!(vdp.bg0_ref_x, 0x00561234);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0RefX as u32), 0x1234);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0RefX as u32 + 2), 0x0056);

        // Test RefY
        vdp.write_reg(VdpRegister::Bg0RefY as u32, 0xABCD); // Low word
        vdp.write_reg(VdpRegister::Bg0RefY as u32 + 2, 0x00EF); // High byte

        assert_eq!(vdp.bg0_ref_y, 0x00EFABCD);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0RefY as u32), 0xABCD);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0RefY as u32 + 2), 0x00EF);
    }

    #[test]
    fn vdp_bg0_tilemap_address() {
        let mut vdp = Vdp::new();

        // Test tilemap address register
        vdp.write_reg(VdpRegister::Bg0TilemapAddr as u32, 0x2000);
        assert_eq!(vdp.bg0_tilemap_addr, 0x2000);
        assert_eq!(vdp.read_reg(VdpRegister::Bg0TilemapAddr as u32), 0x2000);
    }

    #[test]
    fn vdp_bg0_affine_control_flag() {
        let mut vdp = Vdp::new();

        // Test affine mode flag
        vdp.write_reg(
            VdpRegister::Bg0Control as u32,
            BgControl::ENABLE.bits() | BgControl::AFFINE.bits(),
        );

        assert!(vdp.bg0_control.contains(BgControl::ENABLE));
        assert!(vdp.bg0_control.contains(BgControl::AFFINE));
    }

    #[test]
    fn vdp_bg0_identity_transformation() {
        let mut vdp = Vdp::new();

        // Set up a simple test case with identity transformation
        vdp.set_display_enable(true);
        vdp.set_layer_enable(true, false, false);

        // Enable BG0 with affine mode
        vdp.write_reg(
            VdpRegister::Bg0Control as u32,
            BgControl::ENABLE.bits() | BgControl::AFFINE.bits(),
        );

        // Identity matrix (1.0 scale, no rotation) - 8.8 fixed point
        vdp.write_reg(VdpRegister::Bg0AffineA as u32, 0x0100); // 1.0
        vdp.write_reg(VdpRegister::Bg0AffineB as u32, 0x0000); // 0.0
        vdp.write_reg(VdpRegister::Bg0AffineC as u32, 0x0000); // 0.0
        vdp.write_reg(VdpRegister::Bg0AffineD as u32, 0x0100); // 1.0

        // Set reference point to center (in 8.8 fixed point)
        vdp.write_reg(VdpRegister::Bg0RefX as u32, 0x0000);
        vdp.write_reg(VdpRegister::Bg0RefX as u32 + 2, 0x0000);
        vdp.write_reg(VdpRegister::Bg0RefY as u32, 0x0000);
        vdp.write_reg(VdpRegister::Bg0RefY as u32 + 2, 0x0000);

        // Set tilemap address
        vdp.write_reg(VdpRegister::Bg0TilemapAddr as u32, 0x0000);

        // Create a simple tile (8x8 red square)
        let mut tile_data = vec![0u8; 64];
        for i in 0..64 {
            tile_data[i] = 1; // Color index 1
        }
        vdp.load_tile_data(0, &tile_data);

        // Set up a simple palette
        let colors = vec![
            (0x00, 0x00, 0x00), // 0: Black (transparent)
            (0x3F, 0x00, 0x00), // 1: Red
        ];
        vdp.load_palette(0, &colors);

        // Set up tilemap (tile 0, palette 0)
        for i in 0..(32 * 32) {
            vdp.write_vram(i * 2, 0x00);
            vdp.write_vram(i * 2 + 1, 0x00);
        }

        // Render a frame
        let cycles_per_frame = Vdp::CYCLES_PER_SCANLINE * Vdp::SCANLINES_PER_FRAME as u64;
        vdp.step(cycles_per_frame);

        // Check that rendering was attempted (framebuffer should have some non-zero pixels)
        let fb = vdp.framebuffer();
        // With identity transformation, the background should be rendered
        // We just verify the function doesn't panic
        assert_eq!(fb.len(), Vdp::NATIVE_WIDTH * Vdp::NATIVE_HEIGHT);
    }

    #[test]
    fn vdp_bg0_non_affine_mode() {
        let mut vdp = Vdp::new();

        // Test BG0 in non-affine mode (simple scrolling)
        vdp.set_display_enable(true);
        vdp.set_layer_enable(true, false, false);

        // Enable BG0 without affine mode
        vdp.write_reg(VdpRegister::Bg0Control as u32, BgControl::ENABLE.bits());

        // Set scroll values
        vdp.write_reg(VdpRegister::Bg0ScrollX as u32, 10);
        vdp.write_reg(VdpRegister::Bg0ScrollY as u32, 20);

        // Set tilemap address
        vdp.write_reg(VdpRegister::Bg0TilemapAddr as u32, 0x0000);

        // Create a simple tile
        let mut tile_data = vec![0u8; 64];
        for i in 0..64 {
            tile_data[i] = 1; // Color index 1
        }
        vdp.load_tile_data(0, &tile_data);

        // Set up palette
        let colors = vec![
            (0x00, 0x00, 0x00), // 0: Black (transparent)
            (0x00, 0x3F, 0x00), // 1: Green
        ];
        vdp.load_palette(0, &colors);

        // Set up tilemap
        for i in 0..(32 * 32) {
            vdp.write_vram(i * 2, 0x00);
            vdp.write_vram(i * 2 + 1, 0x00);
        }

        // Render a frame
        let cycles_per_frame = Vdp::CYCLES_PER_SCANLINE * Vdp::SCANLINES_PER_FRAME as u64;
        vdp.step(cycles_per_frame);

        // Verify the function completes without panicking
        let fb = vdp.framebuffer();
        assert_eq!(fb.len(), Vdp::NATIVE_WIDTH * Vdp::NATIVE_HEIGHT);
    }
}
