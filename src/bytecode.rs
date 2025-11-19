// src/bytecode.rs
// Minimal bytecode module implementation based on the provided schema

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

/// Tagged 32‑bit value used by the VM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Int24(i32),
    Fixed16_16(i32),
    Bool(bool),
    Handle(u16), // simple handle index
    Nil,
}

/// Header of a .bpx file
#[derive(Debug)]
struct Header {
    magic: [u8; 4],
    version: u16,
    flags: u16,
    cp_offset: u32,
    code_offset: u32,
    meta_offset: u32,
    entry_point: u16,
    crc32: u32,
}

/// Represents a parsed bytecode module
#[derive(Debug)]
pub struct BytecodeModule {
    /// Parsed header
    header: Header,
    /// Constant pool values (only numbers for now)
    constants: Vec<Value>,
    /// Raw bytecode section
    code: Vec<u8>,
    /// Entry point function index
    entry_point: u16,
}

impl BytecodeModule {
    /// Load a .bpx file from disk
    pub fn from_file(path: &PathBuf) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        if buf.len() < 23 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File too short for BPX header",
            ));
        }
        // Basic validation of header magic
        if &buf[0..4] != b"BPX0" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid BPX header",
            ));
        }
        let version = u16::from_le_bytes([buf[4], buf[5]]);
        let flags = u16::from_le_bytes([buf[6], buf[7]]);
        let cp_offset = Self::read_u24_le(&buf, 8);
        let code_offset = Self::read_u24_le(&buf, 11);
        let meta_offset = Self::read_u24_le(&buf, 14);
        let entry_point = u16::from_le_bytes([buf[17], buf[18]]);
        let crc32 = u32::from_le_bytes([buf[19], buf[20], buf[21], buf[22]]);
        let header = Header {
            magic: [buf[0], buf[1], buf[2], buf[3]],
            version,
            flags,
            cp_offset,
            code_offset,
            meta_offset,
            entry_point,
            crc32,
        };
        // Constant pool section
        let constants_bytes = &buf[cp_offset as usize..code_offset as usize];
        let constants = Self::parse_constants(constants_bytes);
        // Code section
        let code = if meta_offset > 0 && meta_offset as usize <= buf.len() {
            buf[code_offset as usize..meta_offset as usize].to_vec()
        } else {
            buf[code_offset as usize..].to_vec()
        };
        Ok(Self {
            header,
            constants,
            code,
            entry_point,
        })
    }

    fn read_u24_le(buf: &[u8], offset: usize) -> u32 {
        let b0 = buf[offset] as u32;
        let b1 = buf[offset + 1] as u32;
        let b2 = buf[offset + 2] as u32;
        (b2 << 16) | (b1 << 8) | b0
    }

    fn parse_constants(bytes: &[u8]) -> Vec<Value> {
        let mut v = Vec::new();
        let mut i = 0;
        while i + 3 <= bytes.len() {
            let b0 = bytes[i] as i32;
            let b1 = bytes[i + 1] as i32;
            let b2 = bytes[i + 2] as i32;
            let mut val = (b2 << 16) | (b1 << 8) | b0;
            if val & 0x800000 != 0 {
                val -= 0x1000000;
            }
            v.push(Value::Int24(val));
            i += 3;
        }
        v
    }

    /// Return raw bytecode slice
    pub fn bytecode(&self) -> &[u8] {
        &self.code
    }
}
