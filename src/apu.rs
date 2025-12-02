// Copyright (C) 2025 Dayton Fishell
// Nexel-24 Game Console Emulator
// This file is part of Nexel-24.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version. See the LICENSE file in the project root for details.
// SPDX-License-Identifier: GPL-3.0-or-later

//! APU-6 audio processor control registers and channel handling

use bitflags::bitflags;

/// Number of audio channels supported by APU-6
pub const APU_CHANNEL_COUNT: usize = 6;

/// Each channel has a fixed window of registers
const CHANNEL_STRIDE: u32 = 0x10;
const STATUS_OFFSET: u32 = (APU_CHANNEL_COUNT as u32) * CHANNEL_STRIDE;
const GLOBAL_CONTROL_OFFSET: u32 = STATUS_OFFSET + 0x01;
const GLOBAL_VERSION_OFFSET: u32 = STATUS_OFFSET + 0x02;
const SUPPORTED_VERSION: u8 = 0x10;

bitflags! {
    struct StatusFlags: u8 {
        const BUFFER_EMPTY = 0x01;
        const CHANNEL_ACTIVE = 0x02;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct EffectMask: u8 {
        const ECHO = 0x01;
        const CHORUS = 0x02;
        const EQ = 0x04;
    }
}

/// Voice types supported per channel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelVoice {
    Pcm = 0,
    Fm = 1,
    Wavetable = 2,
    Noise = 3,
}

impl ChannelVoice {
    fn from_bits(bits: u8) -> Self {
        match bits & 0x03 {
            0 => ChannelVoice::Pcm,
            1 => ChannelVoice::Fm,
            2 => ChannelVoice::Wavetable,
            3 => ChannelVoice::Noise,
            _ => ChannelVoice::Pcm,
        }
    }

    fn bits(self) -> u8 {
        match self {
            ChannelVoice::Pcm => 0,
            ChannelVoice::Fm => 1,
            ChannelVoice::Wavetable => 2,
            ChannelVoice::Noise => 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ChannelState {
    enabled: bool,
    voice: ChannelVoice,
    volume: u8,
    pan: u8,
    frequency: u16,
    effect: EffectMask,
    sample_address: u32,
    sample_length: u16,
    buffer_empty: bool,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            enabled: false,
            voice: ChannelVoice::Pcm,
            volume: 0xFF,
            pan: 0x80,
            frequency: 0,
            effect: EffectMask::empty(),
            sample_address: 0,
            sample_length: 0,
            buffer_empty: true,
        }
    }
}

/// Software representation of the APU-6 subsystem
pub struct Apu {
    channels: [ChannelState; APU_CHANNEL_COUNT],
    status: StatusFlags,
    global_control: u8,
    buffer_empty_latch: bool,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            channels: [ChannelState::default(); APU_CHANNEL_COUNT],
            status: StatusFlags::BUFFER_EMPTY,
            global_control: 0,
            buffer_empty_latch: false,
        }
    }

    fn channel_index(offset: u32) -> Option<(usize, u32)> {
        if offset < STATUS_OFFSET {
            let idx = (offset / CHANNEL_STRIDE) as usize;
            if idx < APU_CHANNEL_COUNT {
                return Some((idx, offset % CHANNEL_STRIDE));
            }
        }
        None
    }

    fn update_status(&mut self) {
        let mut flags = StatusFlags::empty();
        if self.channels.iter().any(|chan| chan.enabled) {
            flags.insert(StatusFlags::CHANNEL_ACTIVE);
        }
        if self
            .channels
            .iter()
            .any(|chan| chan.buffer_empty && chan.enabled)
        {
            flags.insert(StatusFlags::BUFFER_EMPTY);
        }
        self.status = flags;
    }

    fn write_channel(&mut self, idx: usize, reg: u32, value: u8) {
        let channel = &mut self.channels[idx];
        match reg {
            0 => {
                channel.enabled = value & 0x01 != 0;
                channel.voice = ChannelVoice::from_bits((value >> 1) & 0x03);
                if channel.enabled {
                    channel.buffer_empty = channel.sample_length == 0;
                }
            }
            1 => channel.volume = value,
            2 => channel.pan = value,
            3 => {
                if value & 0x01 != 0 {
                    channel.buffer_empty = false;
                }
            }
            4 => channel.frequency = (channel.frequency & 0xFF00) | value as u16,
            5 => channel.frequency = (channel.frequency & 0x00FF) | ((value as u16) << 8),
            6 => channel.effect = EffectMask::from_bits_truncate(value),
            8 => channel.sample_address = (channel.sample_address & 0xFFFF_FF00) | value as u32,
            9 => {
                channel.sample_address =
                    (channel.sample_address & 0xFFFF_00FF) | ((value as u32) << 8)
            }
            10 => {
                channel.sample_address =
                    (channel.sample_address & 0xFF00_FFFF) | ((value as u32) << 16)
            }
            11 => {
                channel.sample_length = (channel.sample_length & 0x00FF) | ((value as u16) << 8);
                if channel.sample_length == 0 && channel.enabled {
                    channel.buffer_empty = true;
                    self.buffer_empty_latch = true;
                } else if channel.sample_length != 0 {
                    channel.buffer_empty = false;
                }
            }
            12 => {
                channel.sample_length = (channel.sample_length & 0xFF00) | value as u16;
                if channel.sample_length == 0 && channel.enabled {
                    channel.buffer_empty = true;
                    self.buffer_empty_latch = true;
                } else if channel.sample_length != 0 {
                    channel.buffer_empty = false;
                }
            }
            _ => {}
        }
        self.update_status();
    }

    fn read_channel(&self, idx: usize, reg: u32) -> u8 {
        let channel = self.channels[idx];
        match reg {
            0 => {
                let mut value = 0;
                if channel.enabled {
                    value |= 0x01;
                }
                value | (channel.voice.bits() << 1)
            }
            1 => channel.volume,
            2 => channel.pan,
            3 => {
                let mut status = 0;
                if channel.buffer_empty {
                    status |= 0x01;
                }
                if channel.enabled {
                    status |= 0x02;
                }
                status
            }
            4 => (channel.frequency & 0x00FF) as u8,
            5 => (channel.frequency >> 8) as u8,
            6 => channel.effect.bits(),
            8 => (channel.sample_address & 0xFF) as u8,
            9 => ((channel.sample_address >> 8) & 0xFF) as u8,
            10 => ((channel.sample_address >> 16) & 0xFF) as u8,
            11 => (channel.sample_length >> 8) as u8,
            12 => (channel.sample_length & 0x00FF) as u8,
            _ => 0xFF,
        }
    }

    /// Read a byte from the register map relative to 0x10C000.
    pub fn read_register(&self, offset: u32) -> u8 {
        if let Some((index, reg_offset)) = Self::channel_index(offset) {
            return self.read_channel(index, reg_offset);
        }
        match offset {
            STATUS_OFFSET => self.status.bits(),
            GLOBAL_CONTROL_OFFSET => self.global_control,
            GLOBAL_VERSION_OFFSET => SUPPORTED_VERSION,
            _ => 0xFF,
        }
    }

    /// Write a byte to the register map relative to 0x10C000.
    pub fn write_register(&mut self, offset: u32, value: u8) {
        if let Some((index, reg_offset)) = Self::channel_index(offset) {
            self.write_channel(index, reg_offset, value);
            return;
        }
        match offset {
            STATUS_OFFSET => {
                if value & 0x01 != 0 {
                    self.buffer_empty_latch = false;
                    self.channels
                        .iter_mut()
                        .for_each(|chan| chan.buffer_empty = false);
                }
            }
            GLOBAL_CONTROL_OFFSET => {
                self.global_control = value;
            }
            _ => {}
        }
    }

    /// Advance audio processing by the specified number of CPU cycles.
    pub fn step(&mut self, cycles: u64) {
        if cycles == 0 {
            return;
        }
        let ticks = (cycles / 64).max(1);
        let mut saw_empty = false;
        for chan in &mut self.channels {
            if !chan.enabled {
                continue;
            }
            if chan.sample_length > 0 {
                let consumed = ticks.min(chan.sample_length as u64) as u16;
                chan.sample_length = chan.sample_length.saturating_sub(consumed);
                if chan.sample_length == 0 {
                    chan.buffer_empty = true;
                    saw_empty = true;
                }
            } else {
                chan.buffer_empty = true;
                saw_empty = true;
            }
        }
        if saw_empty {
            self.buffer_empty_latch = true;
        }
        self.update_status();
    }

    /// Consume the buffer-empty latch and report whether an interrupt should fire.
    pub fn take_buffer_empty(&mut self) -> bool {
        let ready = self.buffer_empty_latch;
        if ready {
            self.buffer_empty_latch = false;
        }
        ready
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_enable_and_voice_setting() {
        let mut apu = Apu::new();
        apu.write_register(0, 0x03);
        assert_eq!(apu.read_register(0) & 0x01, 0x01);
        assert_eq!((apu.read_register(0) >> 1) & 0x03, 0x01);
    }

    #[test]
    fn sample_length_updates_buffer_empty() {
        let mut apu = Apu::new();
        apu.write_register(0, 0x01);
        apu.write_register(12, 0x01);
        assert!(!apu.channels[0].buffer_empty);
        apu.step(64);
        assert!(apu.channels[0].buffer_empty);
    }

    #[test]
    fn buffer_empty_latch_triggers_once() {
        let mut apu = Apu::new();
        apu.write_register(0, 0x01);
        apu.write_register(12, 0x01);
        apu.step(64);
        assert!(apu.take_buffer_empty());
        assert!(!apu.take_buffer_empty());
    }

    #[test]
    fn status_register_reports_flags() {
        let mut apu = Apu::new();
        apu.write_register(0, 0x01);
        apu.write_register(12, 0x00);
        assert_eq!(apu.read_register(STATUS_OFFSET) & 0x03, 0x03);
    }
}
