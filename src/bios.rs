use crate::nraw::assemble;

const BIOS_SIZE: usize = 0x10000;  // 64KB

// Enhanced BIOS with interrupt handlers and system call interface
// Follows BIOS_API.md specification exactly
const BIOS_SOURCE: &str = r#"
; Nexel-24 BIOS
; Interrupt Vector Table at 0xFF0000 (offsets from BIOS base):
;   0x00: RESET            - System reset vector
;   0x03: SWI              - Software interrupt  
;   0x06: PAD_EVENT        - Gamepad event
;   0x09: TIMER0           - Timer 0 overflow
;   0x0C: APU_BUF_EMPTY    - APU buffer empty
;   0x0F: VLU_DONE         - VLU operation complete
;   0x12: DMA_DONE         - DMA transfer complete
;   0x15: HBLANK           - Horizontal blank
;   0x18: NMI              - Non-maskable interrupt

start:
    ; System initialization
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

; Interrupt handlers - all default to RTI
swi_handler:
    RTI

pad_event_handler:
    RTI

timer0_handler:
    RTI

apu_buf_empty_handler:
    RTI

vlu_done_handler:
    RTI

dma_done_handler:
    RTI

hblank_handler:
    RTI

nmi_handler:
    RTI

; System call dispatcher
; Entry point at 0xFF0100
; A register contains system call number (0-2)
; X, Y, R0-R3 contain parameters
; Returns: A = result/status, X = secondary return value
syscall_entry:
    ; Check syscall 0
    BEQ syscall_0
    
    ; Check syscall 1
    DEC A
    BEQ syscall_1
    
    ; Check syscall 2
    DEC A
    BEQ syscall_2
    
    ; Unknown syscall - return error
    LDA #0xFFFF
    RTS

syscall_0:
    ; Syscall 0: Get BIOS version
    ; Returns: A = version number (0x0100 = v1.0)
    LDA #0x0100
    RTS

syscall_1:
    ; Syscall 1: VBlank wait
    ; Wait until next VBlank period begins
vblank_wait_loop:
    LDA 0x100002          ; Read VDP DISPSTAT
    AND #0x0001           ; Check VBlank flag (bit 0)
    BEQ vblank_wait_loop  ; Loop while VBlank bit is 0
    RTS

syscall_2:
    ; Syscall 2: Simple delay
    ; R0 = loop count
    ; Busy-wait delay loop
delay_loop:
    DEC R0
    BNE delay_loop
    RTS
"#;

/// Produce the default BIOS image used by the emulator.
/// Layout matches BIOS_API.md specification:
/// - 0xFF0000-0xFF001B: Interrupt Vector Table (9 vectors × 3 bytes each)
/// - 0xFF0100: System call entry point
/// - 0xFF0200+: BIOS code
pub fn default_bios() -> Vec<u8> {
    let program = assemble(BIOS_SOURCE).expect("invalid BIOS source");
    let mut bios = vec![0xFF; BIOS_SIZE];
    
    // Layout BIOS code starting at offset 0x200 (0xFF0200)
    const BIOS_CODE_OFFSET: usize = 0x200;
    let code_end = BIOS_CODE_OFFSET + program.bytes.len();
    bios[BIOS_CODE_OFFSET..code_end].copy_from_slice(&program.bytes);
    
    // Set up interrupt vector table at 0x00-0x1B (offsets from BIOS base)
    // Each vector is 3 bytes (24-bit address) pointing to handlers
    let vectors = [
        ("start", 0x00),                  // RESET
        ("swi_handler", 0x03),            // SWI
        ("pad_event_handler", 0x06),      // PAD_EVENT
        ("timer0_handler", 0x09),         // TIMER0
        ("apu_buf_empty_handler", 0x0C),  // APU_BUF_EMPTY
        ("vlu_done_handler", 0x0F),       // VLU_DONE
        ("dma_done_handler", 0x12),       // DMA_DONE
        ("hblank_handler", 0x15),         // HBLANK
        ("nmi_handler", 0x18),            // NMI
    ];
    
    for (label, offset) in vectors.iter() {
        if let Some(&label_addr) = program.labels.get(*label) {
            // Calculate absolute 24-bit address: BIOS base + code offset + label offset
            let entry = 0xFF0000 + BIOS_CODE_OFFSET as u32 + label_addr;
            bios[*offset] = (entry & 0xFF) as u8;
            bios[*offset + 1] = ((entry >> 8) & 0xFF) as u8;
            bios[*offset + 2] = ((entry >> 16) & 0xFF) as u8;
        }
    }
    
    // Set up system call entry point at 0x100 (0xFF0100)
    // This is a JMP instruction to the syscall dispatcher
    if let Some(&syscall_addr) = program.labels.get("syscall_entry") {
        let entry = 0xFF0000 + BIOS_CODE_OFFSET as u32 + syscall_addr;
        bios[0x100] = 0x20; // JMP opcode
        bios[0x101] = (entry & 0xFF) as u8;
        bios[0x102] = ((entry >> 8) & 0xFF) as u8;
        bios[0x103] = ((entry >> 16) & 0xFF) as u8;
    }
    
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
        let entry = 0xFF0000 + 0x200 + start_offset;
        
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
            ("pad_event_handler", 0x06),
            ("timer0_handler", 0x09),
            ("apu_buf_empty_handler", 0x0C),
            ("vlu_done_handler", 0x0F),
            ("dma_done_handler", 0x12),
            ("hblank_handler", 0x15),
            ("nmi_handler", 0x18),
        ];
        
        for (label, offset) in handlers.iter() {
            if let Some(&label_addr) = program.labels.get(*label) {
                let entry = 0xFF0000 + 0x200 + label_addr;
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
    
    #[test]
    fn vector_table_layout_correct() {
        let bios = default_bios();
        // Verify vector table spans exactly 0x00-0x1A (27 bytes for 9 vectors)
        // Each vector is 3 bytes
        // RESET at 0x00, SWI at 0x03, ..., NMI at 0x18
        // Last byte of NMI vector is at 0x1A
        
        // Ensure all vector entries are not 0xFF (should be filled in)
        for offset in [0x00, 0x03, 0x06, 0x09, 0x0C, 0x0F, 0x12, 0x15, 0x18] {
            assert_ne!(bios[offset], 0xFF, "Vector at 0x{:02X} not initialized", offset);
        }
    }
}
