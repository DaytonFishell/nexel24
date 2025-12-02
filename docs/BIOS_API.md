# Nexel-24 BIOS API Reference

This document describes the BIOS functions and system calls available on the Nexel-24 console.

## Overview

The Nexel-24 BIOS provides:
- System initialization
- Interrupt handler framework
- System call interface
- Hardware abstraction layer

## Memory Layout

- **BIOS ROM**: 0xFF0000 - 0xFFFFFF (64KB)
- **Interrupt Vector Table**: 0xFF0000 - 0xFF001B
- **System Call Entry**: 0xFF0100

## Interrupt Vector Table

The interrupt vector table starts at 0xFF0000 and contains 24-bit pointers to interrupt handlers:

| Offset | Size | Interrupt        | Priority | Description                |
|--------|------|------------------|----------|----------------------------|
| 0x00   | 3    | RESET            | N/A      | System reset vector        |
| 0x03   | 3    | SWI              | 0        | Software interrupt         |
| 0x06   | 3    | PAD_EVENT        | 1        | Gamepad event              |
| 0x09   | 3    | TIMER0           | 2        | Timer 0 overflow           |
| 0x0C   | 3    | APU_BUF_EMPTY    | 3        | APU buffer empty           |
| 0x0F   | 3    | VLU_DONE         | 4        | VLU operation complete     |
| 0x12   | 3    | DMA_DONE         | 5        | DMA transfer complete      |
| 0x15   | 3    | HBLANK           | 6        | Horizontal blank           |
| 0x18   | 3    | NMI              | 7        | Non-maskable interrupt     |

## System Call Interface

System calls are invoked using the JSR instruction to address 0xFF0100:

```nasm
    LDA #syscall_number
    ; Set up parameters in X, Y, R0-R3 as needed
    JSR 0xFF0100
    ; Result in A (and possibly X)
```

### Calling Convention

- **Input**: 
  - A: System call number
  - X, Y, R0-R3: Parameters (varies by syscall)
- **Output**:
  - A: Return value or status code
  - X: Secondary return value (if applicable)
- **Preserved**: R4-R7 (callee-saved registers)
- **Modified**: A, X, Y, R0-R3 (caller-saved registers)

## System Calls

### Syscall 0: Get BIOS Version

Returns the BIOS version number.

**Parameters**: None

**Returns**:
- A: Version number in format 0xMMmm (major.minor)
  - Example: 0x0100 = version 1.0

**Example**:
```nasm
    LDA #0
    JSR 0xFF0100
    ; A now contains version (e.g., 0x0100)
```

### Syscall 1: VBlank Wait

Waits until the next vertical blank period begins.

**Parameters**: None

**Returns**: None

**Description**: 
Blocks execution until the VBlank flag is set in the VDP's DISPSTAT register. Useful for synchronizing graphics updates with the display refresh.

**Example**:
```nasm
    LDA #1
    JSR 0xFF0100
    ; Execution resumes at start of VBlank
```

### Syscall 2: Delay

Simple busy-wait delay loop.

**Parameters**:
- R0: Loop count (decrements from this value to 0)

**Returns**: None

**Description**:
Executes a simple delay loop. The actual time depends on the CPU clock speed and loop count. Each iteration takes approximately 2 CPU cycles.

**Example**:
```nasm
    LDA #2
    LDX #0x1000      ; Delay count
    MOV X, R0
    JSR 0xFF0100
    ; Delayed for ~8192 cycles
```

## System Initialization

On reset, the BIOS performs the following:

1. **Interrupt Setup**: Enables interrupts (CLI)
2. **VDP Initialization**: Sets DISPCTL to 0x0001 (display enabled)
3. **Cartridge Detection**: Checks for valid cartridge at 0x400000
4. **Boot**: 
   - If cartridge present: Jumps to 0x400000 (cartridge entry point)
   - If no cartridge: Enters idle loop (WFI + branch)

## Interrupt Handlers

All default interrupt handlers in the BIOS simply return immediately (RTI). Games should install their own interrupt handlers by modifying the interrupt vector table or by calling BIOS functions that set up handlers.

### Installing Custom Interrupt Handlers

To install a custom interrupt handler:

```nasm
; Install VBlank handler
    LDA #handler_lo
    STA 0xFF0000+vector_offset
    LDA #handler_mid
    STA 0xFF0000+vector_offset+1
    LDA #handler_hi
    STA 0xFF0000+vector_offset+2
```

**Note**: The BIOS ROM is read-only after initial setup. To change interrupt vectors, you must write to them during cartridge initialization before the BIOS enables ROM protection.

### Interrupt Handler Template

```nasm
my_interrupt_handler:
    ; Save registers you'll modify
    ; (or rely on caller-saved convention)
    
    ; Do interrupt-specific work
    ; ...
    
    ; Restore saved registers
    
    RTI  ; Return from interrupt
```

## Hardware Registers

The BIOS accesses the following hardware registers:

| Address    | Register   | Purpose                           |
|------------|------------|-----------------------------------|
| 0x100000   | VDP_DISPCTL| Display control and enable        |
| 0x100002   | VDP_DISPSTAT| Display status (VBlank, HBlank) |
| 0x400000   | CART_ROM   | Cartridge ROM start address       |

## Cartridge Entry Point

When a valid cartridge is detected, the BIOS jumps to 0x400000. The cartridge should have its entry point at this address:

```nasm
; Cartridge ROM layout
.org 0x400000

cart_entry:
    ; Cartridge initialization
    ; Set up interrupt handlers if needed
    ; Initialize game state
    
    ; Main game loop
main_loop:
    ; Game logic
    BRA main_loop
```

## Best Practices

1. **Always preserve R4-R7** when calling BIOS functions, as they follow the standard calling convention
2. **Use VBlank Wait** (syscall 1) for smooth graphics updates
3. **Install interrupt handlers early** during cartridge initialization
4. **Don't rely on BIOS for real-time tasks** - install your own interrupt handlers for time-critical operations
5. **Check BIOS version** if using extended features that may not exist in older versions

## Error Codes

Some BIOS functions return status codes in the A register:
- 0x0000: Success
- 0xFFFF: Error/Unknown syscall

## Future Extensions

Future BIOS versions may add:
- Memory management functions
- File I/O abstraction
- Audio helper functions
- Network/link cable support
- Debug/trace facilities

Check the BIOS version (syscall 0) to determine available features.

## Example: Complete Cartridge Initialization

```nasm
.org 0x400000

entry:
    ; Disable interrupts during setup
    SEI
    
    ; Check BIOS version
    LDA #0
    JSR 0xFF0100
    ; A now has BIOS version
    
    ; Initialize game state
    JSR init_game
    
    ; Install VBlank handler
    ; (Implementation depends on whether BIOS allows vector modification)
    
    ; Enable interrupts
    CLI
    
    ; Main game loop
main_loop:
    ; Wait for VBlank
    LDA #1
    JSR 0xFF0100
    
    ; Update game state
    JSR update_game
    
    ; Render graphics
    JSR render_frame
    
    BRA main_loop

init_game:
    ; Initialize game variables
    ; Load assets
    ; etc.
    RTS

update_game:
    ; Game logic
    RTS

render_frame:
    ; Draw to VDP
    RTS
```
