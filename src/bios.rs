use crate::nraw::assemble;

const BIOS_SIZE: usize = 0x10000;
const BIOS_CODE_OFFSET: usize = 0x20;

// Enhanced BIOS with interrupt handlers and system call interface
const BIOS_SOURCE: &str = r#"
; Nexel-24 BIOS
; Interrupt Vector Table at 0xFF0000:
;   0x00: Reset vector (points to start)
;   0x03: Software Interrupt (SWI) 
;   0x06: PAD_EVENT
;   0x09: TIMER0
;   0x0C: APU_BUF_EMPTY
;   0x0F: VLU_DONE
;   0x12: DMA_DONE
;   0x15: HBLANK
;   0x18: NMI

start:
    ; Initialize system
    CLI                    ; Enable interrupts
    
    ; Initialize VDP - enable display
    LDA #0x0001
    STA 0x100000          ; VDP DISPCTL register
    
    ; Check for cartridge ROM
    LDA 0x400000          ; Read first word of cart ROM
    ; If cart is present and valid, it should have a non-zero entry
    BNE jump_to_cart
    
idle_loop:
    ; No valid cartridge - just idle and wait for interrupts
    WFI
    BRA idle_loop

jump_to_cart:
    ; Jump to cartridge entry point
    JMP 0x400000

; System call interface
; Entry point: 0xFF0100
; A register contains system call number
; X, Y, R0-R3 contain parameters
syscall_entry:
    ; Dispatch based on syscall number
    ; Check if syscall 0
    BNE check_syscall_1
    JMP syscall_0
    
check_syscall_1:
    ; Decrement and check if syscall 1
    DEC A
    BNE check_syscall_2
    JMP syscall_1
    
check_syscall_2:
    ; Decrement and check if syscall 2
    DEC A
    BNE unknown_syscall
    JMP syscall_2

unknown_syscall:
    LDA #0xFFFF           ; Return error code
    RTS

syscall_0:
    ; Syscall 0: Get BIOS version
    ; Returns: A = version number (0x0100 = v1.0)
    LDA #0x0100
    RTS

syscall_1:
    ; Syscall 1: VBlank wait
    ; Wait until next VBlank
vblank_wait_loop:
    LDA 0x100002          ; Read VDP DISPSTAT
    AND #0x0001           ; Check VBlank flag (bit 0)
    BEQ vblank_wait_loop  ; Loop while VBlank bit is 0
    RTS

syscall_2:
    ; Syscall 2: Simple delay
    ; R0 = loop count
delay_loop:
    DEC R0
    BNE delay_loop
    RTS

; Interrupt handlers
swi_handler:
    ; Software interrupt - currently just return
    RTI

pad_event_handler:
    ; Gamepad event handler
    RTI

timer0_handler:
    ; Timer 0 handler
    RTI

apu_buf_empty_handler:
    ; APU buffer empty handler
    RTI

vlu_done_handler:
    ; VLU operation complete handler
    RTI

dma_done_handler:
    ; DMA complete handler
    RTI

hblank_handler:
    ; HBlank handler
    RTI

nmi_handler:
    ; Non-maskable interrupt handler
    ; Save critical state if needed
    RTI
"#;

/// Produce the default BIOS image used by the emulator.
pub fn default_bios() -> Vec<u8> {
    let program = assemble(BIOS_SOURCE).expect("invalid BIOS source");
    let mut bios = vec![0xFF; BIOS_SIZE];
    
    // Set up interrupt vector table
    let vectors = [
        ("start", 0x00),           // Reset vector
        ("swi_handler", 0x03),     // SWI
        ("pad_event_handler", 0x06), // PAD_EVENT
        ("timer0_handler", 0x09),  // TIMER0
        ("apu_buf_empty_handler", 0x0C), // APU_BUF_EMPTY
        ("vlu_done_handler", 0x0F), // VLU_DONE
        ("dma_done_handler", 0x12), // DMA_DONE
        ("hblank_handler", 0x15),  // HBLANK
        ("nmi_handler", 0x18),     // NMI
    ];
    
    for (label, offset) in vectors.iter() {
        if let Some(&label_addr) = program.labels.get(*label) {
            let entry = 0xFF0000 + BIOS_CODE_OFFSET as u32 + label_addr;
            bios[*offset] = (entry & 0xFF) as u8;
            bios[*offset + 1] = ((entry >> 8) & 0xFF) as u8;
            bios[*offset + 2] = ((entry >> 16) & 0xFF) as u8;
        }
    }
    
    // Set up system call entry point at 0x100 (0xFF0100)
    if let Some(&syscall_addr) = program.labels.get("syscall_entry") {
        let entry = 0xFF0000 + BIOS_CODE_OFFSET as u32 + syscall_addr;
        bios[0x100] = 0x20; // JMP opcode
        bios[0x101] = (entry & 0xFF) as u8;
        bios[0x102] = ((entry >> 8) & 0xFF) as u8;
        bios[0x103] = ((entry >> 16) & 0xFF) as u8;
    }
    
    let code_end = BIOS_CODE_OFFSET + program.bytes.len();
    bios[BIOS_CODE_OFFSET..code_end].copy_from_slice(&program.bytes);
    bios
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bios_is_sized() {
        assert_eq!(default_bios().len(), BIOS_SIZE);
    }

    #[test]
    fn reset_vector_points_to_start() {
        let bios = default_bios();
        let program = assemble(BIOS_SOURCE).expect("assemble BIOS source");
        let start_offset = program.labels.get("start").copied().unwrap_or(0);
        let entry = 0xFF0000 + BIOS_CODE_OFFSET as u32 + start_offset;
        
        // Check reset vector (offset 0x00)
        assert_eq!(bios[0x00], (entry & 0xFF) as u8);
        assert_eq!(bios[0x01], ((entry >> 8) & 0xFF) as u8);
        assert_eq!(bios[0x02], ((entry >> 16) & 0xFF) as u8);
    }
    
    #[test]
    fn interrupt_vectors_are_set() {
        let bios = default_bios();
        let program = assemble(BIOS_SOURCE).expect("assemble BIOS source");
        
        // Verify that interrupt handlers exist and vectors point to them
        let handlers = [
            ("swi_handler", 0x03),
            ("nmi_handler", 0x18),
        ];
        
        for (label, offset) in handlers.iter() {
            if let Some(&label_addr) = program.labels.get(*label) {
                let entry = 0xFF0000 + BIOS_CODE_OFFSET as u32 + label_addr;
                assert_eq!(bios[*offset], (entry & 0xFF) as u8);
                assert_eq!(bios[*offset + 1], ((entry >> 8) & 0xFF) as u8);
                assert_eq!(bios[*offset + 2], ((entry >> 16) & 0xFF) as u8);
            }
        }
    }
    
    #[test]
    fn syscall_entry_exists() {
        let bios = default_bios();
        // Verify syscall entry point at 0x100 has a JMP instruction
        assert_eq!(bios[0x100], 0x20); // JMP opcode
    }
}
