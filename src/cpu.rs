//! HXC-24 CPU subsystem
//!
//! The HXC-24 is a 24-bit address, 16-bit data CPU with:
//! - 18.432MHz clock
//! - 2-stage pipeline
//! - 8 general-purpose registers + special registers (A, X, Y, SP, PC, SR)
//! - Memory-mapped coprocessor access

use crate::core::Bus24;

/// CPU Status Register flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusFlags {
    pub carry: bool,
    pub zero: bool,
    pub interrupt_disable: bool,
    pub decimal: bool,
    pub overflow: bool,
    pub negative: bool,
}

impl StatusFlags {
    pub fn new() -> Self {
        Self {
            carry: false,
            zero: false,
            interrupt_disable: false,
            decimal: false,
            overflow: false,
            negative: false,
        }
    }

    /// Convert flags to u8 representation
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.carry {
            byte |= 0x01;
        }
        if self.zero {
            byte |= 0x02;
        }
        if self.interrupt_disable {
            byte |= 0x04;
        }
        if self.decimal {
            byte |= 0x08;
        }
        if self.overflow {
            byte |= 0x40;
        }
        if self.negative {
            byte |= 0x80;
        }
        byte
    }

    /// Load flags from u8 representation
    pub fn from_byte(byte: u8) -> Self {
        Self {
            carry: (byte & 0x01) != 0,
            zero: (byte & 0x02) != 0,
            interrupt_disable: (byte & 0x04) != 0,
            decimal: (byte & 0x08) != 0,
            overflow: (byte & 0x40) != 0,
            negative: (byte & 0x80) != 0,
        }
    }

    /// Update zero and negative flags based on a 16-bit result
    pub fn update_zn(&mut self, value: u16) {
        self.zero = value == 0;
        self.negative = (value & 0x8000) != 0;
    }
}

impl Default for StatusFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// HXC-24 CPU
pub struct Cpu {
    // Special registers
    pub a: u16,          // Accumulator
    pub x: u16,          // X index register
    pub y: u16,          // Y index register
    pub sp: u16,         // Stack pointer
    pub pc: u32,         // Program counter (24-bit)
    pub sr: StatusFlags, // Status register

    // General purpose registers R0-R7
    pub r: [u16; 8],

    // Cycle counter
    pub cycles: u64,

    // Halted state
    pub halted: bool,

    // Add pending interrupt queue and interrupt handling
    pub pending_interrupts: Vec<u8>,
}

impl Cpu {
    // Interrupt priority constants (higher value = higher priority)
    const NMI_PRIORITY: u8 = 7;
    const HBLANK_PRIORITY: u8 = 6;
    const DMA_DONE_PRIORITY: u8 = 5;
    const VLU_DONE_PRIORITY: u8 = 4;
    const APU_BUF_EMPTY_PRIORITY: u8 = 3;
    const TIMER0_PRIORITY: u8 = 2;
    const PAD_EVENT_PRIORITY: u8 = 1;
    const SWI_PRIORITY: u8 = 0;

    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFFFF,   // Stack grows down from top of WorkRAM
            pc: 0xFF0000, // Start at BIOS
            sr: StatusFlags::new(),
            r: [0; 8],
            cycles: 0,
            halted: false,
            pending_interrupts: Vec::new(),
        }
    }

    /// Reset the CPU to initial state
    pub fn reset(&mut self, bus: &Bus24) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFFFF;
        self.r = [0; 8];
        self.sr = StatusFlags::new();
        self.halted = false;

        // Load reset vector from BIOS (0xFF0000)
        self.pc = bus.read_u24(0xFF0000);
        self.cycles = 0;
    }

    /// Request an interrupt (adds to pending list)
    pub fn request_interrupt(&mut self, int: u8) {
        // If interrupt is maskable and interrupts are disabled, ignore
        if int != 7 && self.sr.interrupt_disable {
            return;
        }
        // Simple deduplication: only add if not already pending
        if !self.pending_interrupts.contains(&int) {
            self.pending_interrupts.push(int);
            // Keep list sorted by priority descending (highest priority first)
            // We reverse the sort order by negating the priority
            self.pending_interrupts.sort_by_key(|&i| {
                let priority = match i {
                    0 => Self::SWI_PRIORITY,
                    1 => Self::PAD_EVENT_PRIORITY,
                    2 => Self::TIMER0_PRIORITY,
                    3 => Self::APU_BUF_EMPTY_PRIORITY,
                    4 => Self::VLU_DONE_PRIORITY,
                    5 => Self::DMA_DONE_PRIORITY,
                    6 => Self::HBLANK_PRIORITY,
                    7 => Self::NMI_PRIORITY,
                    _ => 0,
                };
                // Negate to get descending order (highest priority first)
                std::cmp::Reverse(priority)
            });
        }
    }

    /// Trigger a non‑maskable interrupt (NMI)
    pub fn trigger_nmi(&mut self) {
        // NMI has highest priority (7) and will be sorted to front of queue
        // Insert at position 0 to ensure immediate priority
        if !self.pending_interrupts.contains(&7) {
            self.pending_interrupts.insert(0, 7);
        }
    }

    // Handle highest priority pending interrupt if interrupts are enabled
    // Returns true if an interrupt was handled
    fn handle_interrupts(&mut self, bus: &mut Bus24) -> bool {
        // Check if there's a pending interrupt
        if let Some(&int) = self.pending_interrupts.first() {
            // NMI (interrupt 7) is non-maskable and bypasses interrupt_disable
            // All other interrupts are blocked when interrupt_disable is set
            if int != 7 && self.sr.interrupt_disable {
                return false;
            }
            
            // Simple vector table: each interrupt has a 24-bit address at 0xFF0000 + int*3
            let vector_addr = 0xFF0000 + (int as u32) * 3;
            let handler_addr = bus.read_u24(vector_addr);
            // Push PC onto stack (24-bit)
            self.push_u24(bus, self.pc);
            // Set interrupt disable flag
            self.sr.interrupt_disable = true;
            // Jump to handler
            self.pc = handler_addr;
            // Remove handled interrupt
            self.pending_interrupts.remove(0);
            // Interrupt servicing takes 7 cycles (vector fetch + stack push + jump)
            self.cycles += 7;
            return true;
        }
        false
    }

    /// Execute a single instruction
    pub fn step(&mut self, bus: &mut Bus24) {
        if self.halted {
            self.cycles += 1;
            return;
        }
        // Handle any pending interrupts before fetching next opcode
        // If an interrupt was handled, don't execute an instruction this cycle
        if self.handle_interrupts(bus) {
            return;
        }

        let opcode = bus.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);

        self.execute_instruction(opcode, bus);
    }

    /// Execute an instruction based on opcode
    fn execute_instruction(&mut self, opcode: u8, bus: &mut Bus24) {
        match opcode {
            // NOP - No operation
            0x00 => {
                self.cycles += 1;
            }

            // LDA - Load Accumulator (immediate 16-bit)
            0x01 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.a = value;
                self.sr.update_zn(self.a);
                self.cycles += 2;
            }

            // STA - Store Accumulator (absolute 24-bit address)
            0x02 => {
                let addr = bus.read_u24(self.pc);
                self.pc = self.pc.wrapping_add(3);
                bus.write_u16(addr, self.a);
                self.cycles += 3;
            }

            // LDX - Load X register (immediate 16-bit)
            0x03 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.x = value;
                self.sr.update_zn(self.x);
                self.cycles += 2;
            }

            // STX - Store X register (absolute 24-bit address)
            0x04 => {
                let addr = bus.read_u24(self.pc);
                self.pc = self.pc.wrapping_add(3);
                bus.write_u16(addr, self.x);
                self.cycles += 3;
            }

            // LDY - Load Y register (immediate 16-bit)
            0x05 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.y = value;
                self.sr.update_zn(self.y);
                self.cycles += 2;
            }

            // STY - Store Y register (absolute 24-bit address)
            0x06 => {
                let addr = bus.read_u24(self.pc);
                self.pc = self.pc.wrapping_add(3);
                bus.write_u16(addr, self.y);
                self.cycles += 3;
            }

            // ADD - Add to accumulator (immediate 16-bit)
            0x10 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                let (result, carry) = self.a.overflowing_add(value);
                self.sr.carry = carry;
                self.sr.overflow = ((self.a ^ result) & (value ^ result) & 0x8000) != 0;
                self.a = result;
                self.sr.update_zn(self.a);
                self.cycles += 2;
            }

            // SUB - Subtract from accumulator (immediate 16-bit)
            0x11 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                let (result, borrow) = self.a.overflowing_sub(value);
                // Carry flag is set when no borrow occurs (inverted from the borrow flag)
                self.sr.carry = !borrow;
                self.sr.overflow = ((self.a ^ value) & (self.a ^ result) & 0x8000) != 0;
                self.a = result;
                self.sr.update_zn(self.a);
                self.cycles += 2;
            }

            // AND - Logical AND (immediate 16-bit)
            0x12 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.a &= value;
                self.sr.update_zn(self.a);
                self.cycles += 2;
            }

            // OR - Logical OR (immediate 16-bit)
            0x13 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.a |= value;
                self.sr.update_zn(self.a);
                self.cycles += 2;
            }

            // XOR - Logical XOR (immediate 16-bit)
            0x14 => {
                let value = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.a ^= value;
                self.sr.update_zn(self.a);
                self.cycles += 2;
            }

            // JMP - Jump absolute (24-bit address)
            0x20 => {
                let addr = bus.read_u24(self.pc);
                self.pc = addr;
                self.cycles += 3;
            }

            // JSR - Jump to subroutine (24-bit address)
            0x21 => {
                let addr = bus.read_u24(self.pc);
                self.pc = self.pc.wrapping_add(3);

                // Push return address to stack (24-bit)
                self.push_u24(bus, self.pc);
                self.pc = addr;
                self.cycles += 5;
            }

            // RTS - Return from subroutine
            0x22 => {
                self.pc = self.pop_u24(bus);
                self.cycles += 4;
            }

            // BRA - Branch always (relative 8-bit signed)
            0x30 => {
                // Read 8-bit signed offset from operand, advance past operand, then apply offset
                let offset = bus.read_u8(self.pc) as i8 as i32;
                // Advance PC past the operand byte first (consistent with BEQ/BNE)
                self.pc = self.pc.wrapping_add(1);
                self.pc = self.pc.wrapping_add(offset as u32);
                self.cycles += 2;
            }

            // BEQ - Branch if equal (zero set)
            0x31 => {
                let offset = bus.read_u8(self.pc) as i8 as i32;
                self.pc = self.pc.wrapping_add(1);
                if self.sr.zero {
                    self.pc = self.pc.wrapping_add(offset as u32);
                    self.cycles += 3; // Branch taken adds cycle
                } else {
                    self.cycles += 2;
                }
            }

            // BNE - Branch if not equal (zero clear)
            0x32 => {
                let offset = bus.read_u8(self.pc) as i8 as i32;
                self.pc = self.pc.wrapping_add(1);
                if !self.sr.zero {
                    self.pc = self.pc.wrapping_add(offset as u32);
                    self.cycles += 3; // Branch taken adds cycle
                } else {
                    self.cycles += 2;
                }
            }

            // SEI - Set interrupt disable
            0x40 => {
                self.sr.interrupt_disable = true;
                self.cycles += 1;
            }

            // CLI - Clear interrupt disable
            0x41 => {
                self.sr.interrupt_disable = false;
                self.cycles += 1;
            }

            // RTI - Return from interrupt
            0x42 => {
                self.pc = self.pop_u24(bus);
                self.sr.interrupt_disable = false;
                self.cycles += 5; // Pop takes cycles, similar to RTS
            }

            // HLT - Halt CPU
            0xFF => {
                self.halted = true;
                self.cycles += 1;
            }

            // Unknown opcode - treat as NOP
            _ => {
                self.cycles += 1;
            }
        }
    }

    /// Push a 24-bit value to the stack
    fn push_u24(&mut self, bus: &mut Bus24, value: u32) {
        bus.write_u8(self.sp as u32, (value & 0xFF) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write_u8(self.sp as u32, ((value >> 8) & 0xFF) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write_u8(self.sp as u32, ((value >> 16) & 0xFF) as u8);
        self.sp = self.sp.wrapping_sub(1);
    }

    /// Pop a 24-bit value from the stack
    fn pop_u24(&mut self, bus: &Bus24) -> u32 {
        self.sp = self.sp.wrapping_add(1);
        let hi = bus.read_u8(self.sp as u32) as u32;
        self.sp = self.sp.wrapping_add(1);
        let mid = bus.read_u8(self.sp as u32) as u32;
        self.sp = self.sp.wrapping_add(1);
        let lo = bus.read_u8(self.sp as u32) as u32;
        lo | (mid << 8) | (hi << 16)
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_initialization() {
        let cpu = Cpu::new();
        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.x, 0);
        assert_eq!(cpu.y, 0);
        assert_eq!(cpu.sp, 0xFFFF);
        assert_eq!(cpu.pc, 0xFF0000);
        assert_eq!(cpu.cycles, 0);
        assert!(!cpu.halted);
    }

    #[test]
    fn cpu_reset() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Set up a reset vector
        bus.load_bios(&[0x00, 0x04, 0x40]); // Reset vector: 0x400400

        cpu.a = 0x1234;
        cpu.reset(&bus);

        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.pc, 0x400400);
    }

    #[test]
    fn cpu_lda_immediate() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Load test program in BIOS
        let program = vec![0x01, 0x34, 0x12]; // LDA #0x1234 (little-endian)
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.a, 0x1234);
        assert!(!cpu.sr.zero);
        assert!(!cpu.sr.negative);
        assert_eq!(cpu.cycles, 2);
    }

    #[test]
    fn cpu_sta_absolute() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        cpu.a = 0x5678;

        // STA $1000
        let program = vec![0x02, 0x00, 0x10, 0x00]; // STA $001000 (little-endian 24-bit)
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(bus.read_u16(0x001000), 0x5678);
        assert_eq!(cpu.cycles, 3);
    }

    #[test]
    fn cpu_add_immediate() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        cpu.a = 0x0100;

        // ADD #0x0050
        let program = vec![0x10, 0x50, 0x00]; // ADD #0x0050
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.a, 0x0150);
        assert!(!cpu.sr.zero);
        assert!(!cpu.sr.carry);
        assert!(!cpu.sr.overflow);
    }

    #[test]
    fn cpu_add_overflow() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        cpu.a = 0xFFFF;

        // ADD #0x0001
        let program = vec![0x10, 0x01, 0x00]; // ADD #0x0001
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.a, 0x0000);
        assert!(cpu.sr.zero);
        assert!(cpu.sr.carry);
    }

    #[test]
    fn cpu_sub_immediate() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        cpu.a = 0x0100;

        // SUB #0x0050
        let program = vec![0x11, 0x50, 0x00]; // SUB #0x0050
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.a, 0x00B0);
        assert!(!cpu.sr.zero);
        assert!(cpu.sr.carry); // No borrow
    }

    #[test]
    fn cpu_and_immediate() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        cpu.a = 0xF0F0;

        // AND #0xFF00
        let program = vec![0x12, 0x00, 0xFF]; // AND #0xFF00
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.a, 0xF000);
        assert!(!cpu.sr.zero);
        assert!(cpu.sr.negative); // Bit 15 is set
    }

    #[test]
    fn cpu_jmp_absolute() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // JMP $123456
        let program = vec![0x20, 0x56, 0x34, 0x12]; // JMP $123456
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, 0x123456);
        assert_eq!(cpu.cycles, 3);
    }

    #[test]
    fn cpu_jsr_rts() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        let return_addr = 0xFF0004;

        // JSR $200000, then RTS at 0x200000
        let program = vec![0x21, 0x00, 0x00, 0x20]; // JSR $200000
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        let old_sp = cpu.sp;
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, 0x200000);
        assert_eq!(cpu.sp, old_sp.wrapping_sub(3)); // Stack grew by 3 bytes

        // RTS - write it to VRAM area where we can write
        bus.write_u8(0x200000, 0x22); // RTS opcode
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, return_addr);
        assert_eq!(cpu.sp, old_sp); // Stack restored
    }

    #[test]
    fn cpu_beq_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        cpu.sr.zero = true;

        // BEQ +10
        let program = vec![0x31, 10]; // BEQ +10
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, 0xFF0000 + 2 + 10);
        assert_eq!(cpu.cycles, 3); // Branch taken
    }

    #[test]
    fn cpu_beq_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        cpu.sr.zero = false;

        // BEQ +10
        let program = vec![0x31, 10]; // BEQ +10
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, 0xFF0002);
        assert_eq!(cpu.cycles, 2); // Branch not taken
    }

    #[test]
    fn cpu_halt() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // HLT
        let program = vec![0xFF]; // HLT opcode
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        assert!(!cpu.halted);

        cpu.step(&mut bus);
        assert!(cpu.halted);

        // Further steps should do nothing but increment cycles
        let cycles_before = cpu.cycles;
        cpu.step(&mut bus);
        assert_eq!(cpu.cycles, cycles_before + 1);
    }

    #[test]
    fn status_flags_to_from_byte() {
        let mut flags = StatusFlags::new();
        flags.carry = true;
        flags.zero = true;
        flags.negative = true;

        let byte = flags.to_byte();
        let restored = StatusFlags::from_byte(byte);

        assert_eq!(flags, restored);
    }

    #[test]
    fn interrupt_request_and_service() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Set up BIOS with interrupt vector table
        // Interrupt 4 (VLU_DONE) vector at offset 0x0C (0xFF0000 + 4*3)
        let mut bios = vec![0; 0x100]; // Small BIOS with vectors
        // Set vector for interrupt 4 to point to 0x200000
        bios[0x0C] = 0x00; // Low byte
        bios[0x0D] = 0x00; // Mid byte
        bios[0x0E] = 0x20; // High byte (0x200000)
        // Put a NOP at the start
        bios[0] = 0x00;
        bus.load_bios(&bios);

        cpu.pc = 0xFF0000;
        cpu.sr.interrupt_disable = false;

        // Request interrupt 4 (VLU_DONE)
        cpu.request_interrupt(4);
        assert_eq!(cpu.pending_interrupts.len(), 1);
        assert_eq!(cpu.pending_interrupts[0], 4);

        let old_sp = cpu.sp;
        let old_pc = cpu.pc;

        // Step should handle the interrupt
        cpu.step(&mut bus);

        // Check that PC jumped to handler
        assert_eq!(cpu.pc, 0x200000);

        // Check that old PC was pushed to stack (SP decreased by 3)
        assert_eq!(cpu.sp, old_sp.wrapping_sub(3));
        
        // Verify the pushed value by popping it back
        let mut test_cpu = Cpu::new();
        test_cpu.sp = cpu.sp;
        let popped_pc = test_cpu.pop_u24(&bus);
        assert_eq!(popped_pc, old_pc);

        // Check that interrupt disable flag is set
        assert!(cpu.sr.interrupt_disable);

        // Check that interrupt was removed from queue
        assert_eq!(cpu.pending_interrupts.len(), 0);
    }

    #[test]
    fn interrupt_disabled_when_flag_set() {
        let mut cpu = Cpu::new();

        // Set interrupt disable flag
        cpu.sr.interrupt_disable = true;

        // Request a maskable interrupt
        cpu.request_interrupt(4);

        // Interrupt should not be added to pending queue
        assert_eq!(cpu.pending_interrupts.len(), 0);
    }

    #[test]
    fn nmi_not_maskable() {
        let mut cpu = Cpu::new();

        // Set interrupt disable flag
        cpu.sr.interrupt_disable = true;

        // Trigger NMI (interrupt 7)
        cpu.trigger_nmi();

        // NMI should still be added to pending queue
        assert_eq!(cpu.pending_interrupts.len(), 1);
        assert_eq!(cpu.pending_interrupts[0], 7);
    }

    #[test]
    fn interrupt_priority_ordering() {
        let mut cpu = Cpu::new();

        cpu.sr.interrupt_disable = false;

        // Request multiple interrupts in random order
        cpu.request_interrupt(2); // TIMER0 (priority 2)
        cpu.request_interrupt(4); // VLU_DONE (priority 4)
        cpu.request_interrupt(1); // PAD_EVENT (priority 1)
        cpu.request_interrupt(5); // DMA_DONE (priority 5)

        // Should be sorted by priority (highest first)
        assert_eq!(cpu.pending_interrupts.len(), 4);
        assert_eq!(cpu.pending_interrupts[0], 5); // DMA_DONE (highest)
        assert_eq!(cpu.pending_interrupts[1], 4); // VLU_DONE
        assert_eq!(cpu.pending_interrupts[2], 2); // TIMER0
        assert_eq!(cpu.pending_interrupts[3], 1); // PAD_EVENT (lowest)
    }

    #[test]
    fn nmi_has_highest_priority() {
        let mut cpu = Cpu::new();

        cpu.sr.interrupt_disable = false;

        // Request some interrupts
        cpu.request_interrupt(5); // DMA_DONE
        cpu.request_interrupt(4); // VLU_DONE

        // Trigger NMI
        cpu.trigger_nmi();

        // NMI should be first in queue
        assert_eq!(cpu.pending_interrupts[0], 7);
    }

    #[test]
    fn multiple_interrupts_serviced_in_order() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Set up BIOS with interrupt vectors
        let mut bios = vec![0; 0x100];
        // INT 4 vector at offset 0x0C -> 0x200000
        bios[0x0C] = 0x00;
        bios[0x0D] = 0x00;
        bios[0x0E] = 0x20;
        // INT 5 vector at offset 0x0F -> 0x201000
        bios[0x0F] = 0x00;
        bios[0x10] = 0x10;
        bios[0x11] = 0x20;
        // NOP at start
        bios[0] = 0x00;
        bus.load_bios(&bios);

        cpu.pc = 0xFF0000;
        cpu.sr.interrupt_disable = false;

        // Request two interrupts
        cpu.request_interrupt(4); // Lower priority
        cpu.request_interrupt(5); // Higher priority

        // First step should service INT 5 (higher priority)
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x201000);
        assert_eq!(cpu.pending_interrupts.len(), 1);

        // Re-enable interrupts for next one
        cpu.sr.interrupt_disable = false;

        // Next step should service INT 4
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x200000);
        assert_eq!(cpu.pending_interrupts.len(), 0);
    }

    #[test]
    fn rti_restores_state() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Set up BIOS with interrupt vector
        let mut bios = vec![0; 0x100];
        // INT 4 vector at offset 0x0C -> 0x200000
        bios[0x0C] = 0x00;
        bios[0x0D] = 0x00;
        bios[0x0E] = 0x20;
        // NOP at start
        bios[0] = 0x00;
        bus.load_bios(&bios);

        // Set up handler with RTI instruction at 0x200000
        bus.write_u8(0x200000, 0x42); // RTI opcode

        cpu.pc = 0xFF0000;
        cpu.sr.interrupt_disable = false;

        // Request interrupt
        cpu.request_interrupt(4);

        let original_pc = cpu.pc;
        let original_sp = cpu.sp;

        // Service interrupt
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x200000);
        assert!(cpu.sr.interrupt_disable);

        // Execute RTI
        cpu.step(&mut bus);

        // PC should be restored
        assert_eq!(cpu.pc, original_pc);

        // SP should be restored
        assert_eq!(cpu.sp, original_sp);

        // Interrupt disable should be cleared
        assert!(!cpu.sr.interrupt_disable);
    }

    #[test]
    fn interrupt_not_serviced_when_disabled() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Set up BIOS with a NOP instruction
        let bios = vec![0x00]; // NOP at 0xFF0000
        bus.load_bios(&bios);

        cpu.pc = 0xFF0000;
        cpu.sr.interrupt_disable = true; // Interrupts disabled

        // Request interrupt (through direct manipulation to bypass request_interrupt logic)
        cpu.pending_interrupts.push(4);

        let old_pc = cpu.pc;

        // Step should not service interrupt
        cpu.step(&mut bus);

        // PC should have advanced by NOP, not jumped to handler
        assert_eq!(cpu.pc, old_pc + 1);
        assert_eq!(cpu.pending_interrupts.len(), 1); // Still pending
    }

    #[test]
    fn sei_cli_instructions() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Test SEI (Set interrupt disable)
        let program = vec![
            0x40, // SEI
            0x41, // CLI
        ];
        bus.load_bios(&program);

        cpu.pc = 0xFF0000;
        cpu.sr.interrupt_disable = false;

        // Execute SEI
        cpu.step(&mut bus);
        assert!(cpu.sr.interrupt_disable);

        // Execute CLI
        cpu.step(&mut bus);
        assert!(!cpu.sr.interrupt_disable);
    }

    #[test]
    fn nmi_interrupts_even_when_disabled() {
        let mut cpu = Cpu::new();
        let mut bus = Bus24::new();

        // Set up BIOS with NMI vector
        let mut bios = vec![0; 0x100];
        // NMI (interrupt 7) vector at offset 0x15 (0xFF0000 + 7*3) -> 0x200000
        bios[0x15] = 0x00;
        bios[0x16] = 0x00;
        bios[0x17] = 0x20;
        // NOP at start
        bios[0] = 0x00;
        bus.load_bios(&bios);

        cpu.pc = 0xFF0000;
        cpu.sr.interrupt_disable = true; // Interrupts disabled

        // Trigger NMI
        cpu.trigger_nmi();
        assert_eq!(cpu.pending_interrupts.len(), 1);

        let old_pc = cpu.pc;

        // Step should service NMI even though interrupts are disabled
        cpu.step(&mut bus);

        // PC should have jumped to NMI handler
        assert_eq!(cpu.pc, 0x200000);
        
        // Verify that PC was pushed to stack
        let mut test_cpu = Cpu::new();
        test_cpu.sp = cpu.sp;
        let popped_pc = test_cpu.pop_u24(&bus);
        assert_eq!(popped_pc, old_pc);

        // NMI should be removed from queue
        assert_eq!(cpu.pending_interrupts.len(), 0);
    }

    #[test]
    fn duplicate_interrupt_not_added() {
        let mut cpu = Cpu::new();

        cpu.sr.interrupt_disable = false;

        // Request the same interrupt twice
        cpu.request_interrupt(4);
        cpu.request_interrupt(4);

        // Should only be in queue once
        assert_eq!(cpu.pending_interrupts.len(), 1);
        assert_eq!(cpu.pending_interrupts[0], 4);
    }
}
