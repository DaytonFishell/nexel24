# Nexel-24 NRAW Calling Convention

This document defines the standard calling convention for NRAW assembly language on the Nexel-24 (HX-1) console.

## Register Usage

### Special Purpose Registers
- **A** (Accumulator): Primary arithmetic and logic operations, return value
- **X**: Index register, second return value (for 32-bit results), preserved by caller
- **Y**: Index register, preserved by caller
- **SP** (Stack Pointer): Points to top of stack, grows downward from 0xFFFF
- **PC** (Program Counter): 24-bit instruction pointer
- **SR** (Status Register): Flags (C, Z, I, D, V, N)

### General Purpose Registers
- **R0-R3**: Argument registers (caller-saved)
  - R0: First argument / scratch register
  - R1: Second argument / scratch register
  - R2: Third argument / scratch register
  - R3: Fourth argument / scratch register
- **R4-R7**: Preserved registers (callee-saved)
  - R4-R7: Local variables, must be preserved across function calls

## Function Call Protocol

### Parameter Passing
1. **First 4 parameters**: Pass in R0, R1, R2, R3 (in that order)
2. **Additional parameters**: Push onto stack in reverse order (rightmost first)
3. **16-bit values**: Pass directly in registers or stack
4. **32-bit values**: Use two registers (low word in first register, high word in second)
5. **Structures**: Pass by pointer in R0

### Return Values
- **16-bit return value**: Return in A register
- **32-bit return value**: Low 16 bits in A, high 16 bits in X
- **Pointer return**: Return address in A (16-bit WorkRAM pointer) or full 24-bit in A:X
- **Boolean**: Return 0 (false) or non-zero (true) in A

### Register Preservation
**Caller-saved (volatile)**:
- A, X, Y
- R0, R1, R2, R3

**Callee-saved (non-volatile)**:
- R4, R5, R6, R7
- SP (stack pointer)

Callees must preserve R4-R7 and SP. If a function uses these registers, it must save them on entry and restore them before returning.

## Stack Frame Layout

The stack grows downward from 0xFFFF. A typical stack frame looks like:

```
High Address (0xFFFF)
+------------------+
| Return Address   |  SP+0 to SP+2 (24-bit, pushed by JSR)
| (3 bytes)        |
+------------------+
| Saved R7         |  SP-3 to SP-4 (if used)
+------------------+
| Saved R6         |  SP-5 to SP-6 (if used)
+------------------+
| Saved R5         |  SP-7 to SP-8 (if used)
+------------------+
| Saved R4         |  SP-9 to SP-10 (if used)
+------------------+
| Local Variables  |  SP-11 onwards
+------------------+
| Stack Arguments  |  Additional parameters (if any)
+------------------+
Low Address
```

### Stack Frame Setup (Prologue)
```nasm
function_entry:
    ; Save callee-saved registers if needed
    DEC SP
    MOV R4, A
    STA (SP)      ; Save R4
    DEC SP
    MOV R5, A
    STA (SP)      ; Save R5
    ; ... continue for R6, R7 if needed
    
    ; Allocate space for local variables
    LDA #local_size
    SUB A, SP     ; SP -= local_size
    MOV SP, A
```

### Stack Frame Teardown (Epilogue)
```nasm
function_exit:
    ; Deallocate local variables
    LDA #local_size
    ADD A, SP     ; SP += local_size
    MOV SP, A
    
    ; Restore callee-saved registers
    INC SP
    LDA (SP)
    MOV A, R5     ; Restore R5
    INC SP
    LDA (SP)
    MOV A, R4     ; Restore R4
    ; ... continue for R6, R7 if saved
    
    RTS
```

## Calling Convention Examples

### Example 1: Simple Function Call
```nasm
; Function: add(a, b) -> a + b
; Parameters: a in R0, b in R1
; Returns: result in A

add:
    MOV R0, A      ; Load first parameter
    ADD #R1, A     ; Add second parameter (note: immediate mode for register)
    RTS            ; Return with result in A

; Caller:
caller:
    LDA #10
    MOV A, R0      ; First argument
    LDA #20
    MOV A, R1      ; Second argument
    JSR add        ; Call function
    ; Result now in A
```

### Example 2: Function with Preserved Registers
```nasm
; Function: multiply_and_add(a, b, c) -> (a * b) + c
; Parameters: a in R0, b in R1, c in R2
; Returns: result in A:X (32-bit)

multiply_and_add:
    ; Save callee-saved register we'll use
    DEC SP
    MOV R4, A
    STA (SP)       ; Save R4
    
    ; Function body
    MOV R0, A
    MUL R1, A      ; A:X = R0 * R1
    MOV A, R4      ; Save low word
    MOV X, A
    ADD R2, A      ; Add c to high word
    MOV A, X
    MOV R4, A      ; Move low word to A
    
    ; Restore R4
    INC SP
    LDA (SP)
    MOV A, R4
    
    RTS

; Caller:
caller:
    LDA #100
    MOV A, R0
    LDA #200  
    MOV A, R1
    LDA #50
    MOV A, R2
    JSR multiply_and_add
    ; 32-bit result now in A:X
```

### Example 3: Function with Stack Arguments
```nasm
; Function: sum_five(a, b, c, d, e) -> a + b + c + d + e
; Parameters: a-d in R0-R3, e on stack
; Returns: result in A

sum_five:
    ; Load stack argument (e is at SP+3 after return address)
    LDA (SP+3)     ; Load e
    MOV A, R4      ; Temporarily store in scratch
    
    ; Sum all parameters
    MOV R0, A
    ADD R1, A
    ADD R2, A
    ADD R3, A
    ADD R4, A
    
    RTS

; Caller:
caller:
    LDA #5
    DEC SP
    STA (SP)       ; Push e (5th argument)
    
    LDA #1
    MOV A, R0
    LDA #2
    MOV A, R1
    LDA #3
    MOV A, R2
    LDA #4
    MOV A, R3
    
    JSR sum_five
    
    INC SP         ; Pop stack argument
    ; Result in A
```

## System Call Convention

System calls (BIOS functions) follow a slightly different convention:
- **A register**: System call number
- **X, Y, R0-R3**: Parameters (depending on syscall)
- **Return**: A register (status/result), X may contain additional data
- **Preserved**: R4-R7 are always preserved by BIOS

System calls are invoked using:
```nasm
    LDA #SYSCALL_NUMBER
    ; Set up parameters in X, Y, R0-R3
    JSR 0xFF0100    ; BIOS system call entry point
```

## Best Practices

1. **Always preserve R4-R7** if your function uses them
2. **Use R0-R3 for temporary values** that don't need to survive function calls
3. **Clean up stack arguments** after function returns (caller's responsibility)
4. **Document function interfaces** with comments showing parameters and return values
5. **Align stack to 2-byte boundaries** when possible for efficiency
6. **Use registers for small, frequently-accessed data**
7. **Use stack for local arrays and large structures**

## Leaf Function Optimization

Leaf functions (functions that don't call other functions) can use all registers freely without saving them, as long as they're marked as caller-saved. This can significantly improve performance for simple utility functions.

```nasm
; Leaf function - doesn't need to save any registers
is_even:
    MOV R0, A
    AND #1, A
    BEQ is_even_true
    LDA #0         ; Return false
    RTS
is_even_true:
    LDA #1         ; Return true
    RTS
```

## Variadic Functions

For functions with variable arguments (like printf):
1. Use R0 for argument count
2. Use R1 for pointer to argument array
3. Or push all arguments onto stack with count in R0

## Compatibility Notes

This calling convention is designed to be:
- **Efficient**: Minimize stack operations for common cases
- **Simple**: Easy to understand and implement
- **Flexible**: Works with both simple and complex functions
- **Compatible**: Follows common patterns from other 16-bit architectures

All BIOS functions and standard library routines follow this convention.
