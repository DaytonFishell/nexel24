use std::collections::HashMap;

/// Result of running the assembler.
pub struct AssembledProgram {
    pub bytes: Vec<u8>,
    pub labels: HashMap<String, u32>,
}

/// Errors produced while assembling NRAW source.
#[derive(Debug, PartialEq, Eq)]
pub enum AsmError {
    UnknownInstruction { line: usize, token: String },
    MissingOperand { line: usize, instruction: String },
    UnexpectedOperand { line: usize, instruction: String },
    InvalidNumber { line: usize, operand: String },
    LabelNotFound { name: String },
    DuplicateLabel { line: usize, name: String },
    BranchOutOfRange { label: String, offset: i32 },
    InvalidAddressingMode { line: usize, operand: String },
}

#[derive(Debug, Clone)]
enum AddressingMode {
    Immediate(u32),      // #value
    Absolute(u32),       // address or label
    Indirect,            // (SP) or (reg)
    IndirectOffset(i32), // (SP+offset)
    Register(u8),        // Register encoding
}

#[derive(Debug)]
enum InstructionKind {
    Nop,
    Lda,
    Sta,
    Ldx,
    Stx,
    Ldy,
    Sty,
    Add,
    Sub,
    Cmp,
    And,
    Or,
    Xor,
    Mul,
    Div,
    Mov,
    Inc,
    Dec,
    Bit,
    Bset,
    Bclr,
    Jmp,
    Jsr,
    Rts,
    Bra,
    Beq,
    Bne,
    Bcs,
    Bcc,
    Bmi,
    Bpl,
    Bvs,
    Bvc,
    Sei,
    Cli,
    Rti,
    Wfi,
    Cop,
    Hlt,
}

enum Operand {
    Single(AddressingMode),
    Dual(AddressingMode, AddressingMode), // For MOV src, dst
    Label(String),
}

struct RawInstruction {
    kind: InstructionKind,
    operand: Option<Operand>,
    address: u32,
    line: usize,
}

/// Assemble a small NRAW program into bytes and label positions.
pub fn assemble(source: &str) -> Result<AssembledProgram, AsmError> {
    let mut labels = HashMap::new();
    let mut instructions: Vec<RawInstruction> = Vec::new();
    let mut address = 0u32;

    for (line_idx, line) in source.lines().enumerate() {
        let stripped = line.split(';').next().unwrap_or("").trim();
        if stripped.is_empty() {
            continue;
        }

        let mut working = stripped;
        loop {
            if let Some(colon) = working.find(':') {
                let label = working[..colon].trim().to_string();
                if labels.contains_key(&label) {
                    return Err(AsmError::DuplicateLabel {
                        line: line_idx + 1,
                        name: label,
                    });
                }
                labels.insert(label, address);
                working = working[colon + 1..].trim();
            } else {
                break;
            }
        }

        if working.is_empty() {
            continue;
        }

        // Parse instruction and operands
        let mut parts = working.split_whitespace();
        let op = parts.next().unwrap();
        let name = op.to_uppercase();
        
        // Collect remaining operands (may have comma-separated for two-operand instructions)
        let rest: Vec<&str> = parts.collect();
        let operands_str = rest.join(" ");
        
        // Split by comma for two-operand instructions
        let operand_parts: Vec<&str> = operands_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let kind = match name.as_str() {
            "NOP" => InstructionKind::Nop,
            "LDA" => InstructionKind::Lda,
            "LDX" => InstructionKind::Ldx,
            "LDY" => InstructionKind::Ldy,
            "STA" => InstructionKind::Sta,
            "STX" => InstructionKind::Stx,
            "STY" => InstructionKind::Sty,
            "ADD" => InstructionKind::Add,
            "SUB" => InstructionKind::Sub,
            "CMP" => InstructionKind::Cmp,
            "AND" => InstructionKind::And,
            "OR" => InstructionKind::Or,
            "XOR" => InstructionKind::Xor,
            "MUL" => InstructionKind::Mul,
            "DIV" => InstructionKind::Div,
            "MOV" => InstructionKind::Mov,
            "INC" => InstructionKind::Inc,
            "DEC" => InstructionKind::Dec,
            "BIT" => InstructionKind::Bit,
            "BSET" => InstructionKind::Bset,
            "BCLR" => InstructionKind::Bclr,
            "JMP" => InstructionKind::Jmp,
            "JSR" => InstructionKind::Jsr,
            "RTS" => InstructionKind::Rts,
            "BRA" => InstructionKind::Bra,
            "BEQ" => InstructionKind::Beq,
            "BNE" => InstructionKind::Bne,
            "BCS" => InstructionKind::Bcs,
            "BCC" => InstructionKind::Bcc,
            "BMI" => InstructionKind::Bmi,
            "BPL" => InstructionKind::Bpl,
            "BVS" => InstructionKind::Bvs,
            "BVC" => InstructionKind::Bvc,
            "SEI" => InstructionKind::Sei,
            "CLI" => InstructionKind::Cli,
            "RTI" => InstructionKind::Rti,
            "WFI" => InstructionKind::Wfi,
            "COP" => InstructionKind::Cop,
            "HLT" => InstructionKind::Hlt,
            _ => {
                return Err(AsmError::UnknownInstruction {
                    line: line_idx + 1,
                    token: op.to_string(),
                });
            }
        };

        let operand = parse_operands(&kind, &operand_parts, line_idx + 1)?;
        let inst_length = instruction_length(&kind, &operand);
        
        instructions.push(RawInstruction {
            kind,
            operand,
            address,
            line: line_idx + 1,
        });
        address = address.wrapping_add(inst_length);
    }

    let mut bytes = Vec::with_capacity(address as usize);
    for inst in instructions {
        encode_instruction(&inst, &labels, &mut bytes)?;
    }

    Ok(AssembledProgram { bytes, labels })
}

fn parse_addressing_mode(token: &str, line: usize) -> Result<AddressingMode, AsmError> {
    let token = token.trim();
    
    // Immediate mode: #value
    if let Some(stripped) = token.strip_prefix('#') {
        let value = parse_number(stripped, line)?;
        return Ok(AddressingMode::Immediate(value));
    }
    
    // Indirect modes: (SP), (SP+offset), (reg)
    if token.starts_with('(') && token.ends_with(')') {
        let inner = &token[1..token.len()-1].trim();
        
        // Check for offset: (SP+offset) or (SP-offset)
        if let Some(plus_pos) = inner.find('+') {
            let reg_part = inner[..plus_pos].trim();
            let offset_part = inner[plus_pos+1..].trim();
            if reg_part.to_uppercase() == "SP" {
                let offset = parse_number(offset_part, line)? as i32;
                return Ok(AddressingMode::IndirectOffset(offset));
            }
        } else if let Some(minus_pos) = inner.find('-') {
            let reg_part = inner[..minus_pos].trim();
            let offset_part = inner[minus_pos+1..].trim();
            if reg_part.to_uppercase() == "SP" {
                let offset = -(parse_number(offset_part, line)? as i32);
                return Ok(AddressingMode::IndirectOffset(offset));
            }
        } else {
            // Simple indirect: (SP) or (reg)
            return Ok(AddressingMode::Indirect);
        }
    }
    
    // Try to parse as register
    if let Ok(reg) = parse_register(token, line) {
        return Ok(AddressingMode::Register(reg));
    }
    
    // Try to parse as absolute address
    if let Ok(value) = parse_number(token, line) {
        return Ok(AddressingMode::Absolute(value));
    }
    
    // Otherwise it's an error
    Err(AsmError::InvalidAddressingMode {
        line,
        operand: token.to_string(),
    })
}

fn parse_operands(
    kind: &InstructionKind,
    operand_parts: &[&str],
    line: usize,
) -> Result<Option<Operand>, AsmError> {
    let name = format!("{:?}", kind);
    
    match kind {
        // No operand instructions
        InstructionKind::Nop
        | InstructionKind::Rts
        | InstructionKind::Sei
        | InstructionKind::Cli
        | InstructionKind::Rti
        | InstructionKind::Wfi
        | InstructionKind::Hlt => {
            if !operand_parts.is_empty() {
                return Err(AsmError::UnexpectedOperand {
                    line,
                    instruction: name,
                });
            }
            Ok(None)
        }
        
        // Two-operand instructions: MOV src, dst
        InstructionKind::Mov => {
            if operand_parts.len() != 2 {
                return Err(AsmError::MissingOperand {
                    line,
                    instruction: name,
                });
            }
            let src = parse_addressing_mode(operand_parts[0], line)?;
            let dst = parse_addressing_mode(operand_parts[1], line)?;
            Ok(Some(Operand::Dual(src, dst)))
        }
        
        // Single register operand: INC reg, DEC reg
        InstructionKind::Inc | InstructionKind::Dec => {
            if operand_parts.len() != 1 {
                return Err(AsmError::MissingOperand {
                    line,
                    instruction: name,
                });
            }
            let mode = parse_addressing_mode(operand_parts[0], line)?;
            Ok(Some(Operand::Single(mode)))
        }
        
        // Load/store instructions: can be immediate, absolute, indirect, or register (for indirect)
        InstructionKind::Lda | InstructionKind::Ldx | InstructionKind::Ldy |
        InstructionKind::Sta | InstructionKind::Stx | InstructionKind::Sty => {
            if operand_parts.len() != 1 {
                return Err(AsmError::MissingOperand {
                    line,
                    instruction: name,
                });
            }
            
            let operand_str = operand_parts[0];
            
            // Check if it's a label (not a number, not immediate, not indirect)
            if !operand_str.starts_with('#') && !operand_str.starts_with('(') {
                if let Ok(_) = parse_number(operand_str, line) {
                    Ok(Some(Operand::Single(AddressingMode::Absolute(parse_number(operand_str, line)?))))
                } else {
                    Ok(Some(Operand::Label(operand_str.to_string())))
                }
            } else {
                let mode = parse_addressing_mode(operand_str, line)?;
                Ok(Some(Operand::Single(mode)))
            }
        }
        
        // Arithmetic/logic instructions: can take register or immediate
        InstructionKind::Add | InstructionKind::Sub | InstructionKind::Cmp |
        InstructionKind::And | InstructionKind::Or | InstructionKind::Xor | InstructionKind::Mul |
        InstructionKind::Div | InstructionKind::Bit |
        InstructionKind::Bset | InstructionKind::Bclr => {
            if operand_parts.len() != 1 {
                return Err(AsmError::MissingOperand {
                    line,
                    instruction: name,
                });
            }
            let mode = parse_addressing_mode(operand_parts[0], line)?;
            Ok(Some(Operand::Single(mode)))
        }
        
        // Jump/call instructions: absolute address or label
        InstructionKind::Jmp | InstructionKind::Jsr => {
            if operand_parts.len() != 1 {
                return Err(AsmError::MissingOperand {
                    line,
                    instruction: name,
                });
            }
            
            let operand_str = operand_parts[0];
            
            // Try as number first, otherwise treat as label
            if let Ok(value) = parse_number(operand_str, line) {
                Ok(Some(Operand::Single(AddressingMode::Absolute(value))))
            } else {
                Ok(Some(Operand::Label(operand_str.to_string())))
            }
        }
        
        // Branch instructions: relative offset or label
        InstructionKind::Bra | InstructionKind::Beq | InstructionKind::Bne |
        InstructionKind::Bcs | InstructionKind::Bcc | InstructionKind::Bmi |
        InstructionKind::Bpl | InstructionKind::Bvs | InstructionKind::Bvc => {
            if operand_parts.len() != 1 {
                return Err(AsmError::MissingOperand {
                    line,
                    instruction: name,
                });
            }
            
            let operand_str = operand_parts[0];
            
            // Try as number first, otherwise treat as label
            if let Ok(value) = parse_number(operand_str, line) {
                Ok(Some(Operand::Single(AddressingMode::Absolute(value))))
            } else {
                Ok(Some(Operand::Label(operand_str.to_string())))
            }
        }
        
        // Coprocessor instruction
        InstructionKind::Cop => {
            if operand_parts.len() != 1 {
                return Err(AsmError::MissingOperand {
                    line,
                    instruction: name,
                });
            }
            let mode = parse_addressing_mode(operand_parts[0], line)?;
            Ok(Some(Operand::Single(mode)))
        }
    }
}

fn parse_number(token: &str, line: usize) -> Result<u32, AsmError> {
    if let Some(stripped) = token.strip_prefix("0x") {
        u32::from_str_radix(stripped, 16).map_err(|_| AsmError::InvalidNumber {
            line,
            operand: token.to_string(),
        })
    } else if token.starts_with('$') {
        u32::from_str_radix(&token[1..], 16).map_err(|_| AsmError::InvalidNumber {
            line,
            operand: token.to_string(),
        })
    } else {
        token.parse::<u32>().map_err(|_| AsmError::InvalidNumber {
            line,
            operand: token.to_string(),
        })
    }
}

fn parse_register(token: &str, line: usize) -> Result<u8, AsmError> {
    let upper = token.to_uppercase();
    match upper.as_str() {
        "A" => Ok(0),
        "X" => Ok(1),
        "Y" => Ok(2),
        "SP" => Ok(3),
        "R0" => Ok(4),
        "R1" => Ok(5),
        "R2" => Ok(6),
        "R3" => Ok(7),
        "R4" => Ok(8),
        "R5" => Ok(9),
        "R6" => Ok(10),
        "R7" => Ok(11),
        _ => Err(AsmError::InvalidNumber {
            line,
            operand: token.to_string(),
        }),
    }
}

fn instruction_length(kind: &InstructionKind, operand: &Option<Operand>) -> u32 {
    match kind {
        InstructionKind::Nop
        | InstructionKind::Rts
        | InstructionKind::Sei
        | InstructionKind::Cli
        | InstructionKind::Rti
        | InstructionKind::Wfi
        | InstructionKind::Hlt => 1,
        
        // Branch instructions: 1 byte opcode + 1 byte signed offset
        InstructionKind::Bra | InstructionKind::Beq | InstructionKind::Bne |
        InstructionKind::Bcs | InstructionKind::Bcc | InstructionKind::Bmi |
        InstructionKind::Bpl | InstructionKind::Bvs | InstructionKind::Bvc => 2,
        
        // Register operations and coprocessor: variable based on operand
        InstructionKind::Inc | InstructionKind::Dec | InstructionKind::Cop => 2,
        
        // MOV can be 2 or 3 bytes depending on addressing mode
        InstructionKind::Mov => 3, // opcode + src_reg + dst_reg
        
        // Load/store instructions: depends on addressing mode
        InstructionKind::Lda | InstructionKind::Ldx | InstructionKind::Ldy |
        InstructionKind::Sta | InstructionKind::Stx | InstructionKind::Sty => {
            match operand {
                Some(Operand::Single(AddressingMode::Immediate(_))) => 3,
                Some(Operand::Single(AddressingMode::Absolute(_))) | Some(Operand::Label(_)) => 4,
                Some(Operand::Single(AddressingMode::Indirect)) => 2,
                Some(Operand::Single(AddressingMode::IndirectOffset(_))) => 3,
                _ => 2,
            }
        }
        
        // Arithmetic instructions: typically immediate (3 bytes) or register (2 bytes)
        InstructionKind::Add | InstructionKind::Sub | InstructionKind::Cmp |
        InstructionKind::And | InstructionKind::Or | InstructionKind::Xor | InstructionKind::Mul |
        InstructionKind::Div | InstructionKind::Bit |
        InstructionKind::Bset | InstructionKind::Bclr => {
            match operand {
                Some(Operand::Single(AddressingMode::Immediate(_))) => 3,
                Some(Operand::Single(AddressingMode::Register(_))) => 2,
                _ => 2,
            }
        }
        
        // Jump/call: 1 byte opcode + 3 bytes for 24-bit address
        InstructionKind::Jmp | InstructionKind::Jsr => 4,
    }
}

fn encode_instruction(
    inst: &RawInstruction,
    labels: &HashMap<String, u32>,
    bytes: &mut Vec<u8>,
) -> Result<(), AsmError> {
    match inst.kind {
        InstructionKind::Nop => bytes.push(0x00),
        InstructionKind::Hlt => bytes.push(0xFF),
        InstructionKind::Rts => bytes.push(0x22),
        InstructionKind::Sei => bytes.push(0x40),
        InstructionKind::Cli => bytes.push(0x41),
        InstructionKind::Rti => bytes.push(0x42),
        InstructionKind::Wfi => bytes.push(0x43),
        
        InstructionKind::Lda => encode_load_store(0x01, 0x07, 0x0A, inst, labels, bytes)?,
        InstructionKind::Ldx => encode_load_store(0x03, 0x08, 0x0B, inst, labels, bytes)?,
        InstructionKind::Ldy => encode_load_store(0x05, 0x09, 0x0C, inst, labels, bytes)?,
        InstructionKind::Sta => encode_load_store(0x02, 0x02, 0x0D, inst, labels, bytes)?,
        InstructionKind::Stx => encode_load_store(0x04, 0x04, 0x0E, inst, labels, bytes)?,
        InstructionKind::Sty => encode_load_store(0x06, 0x06, 0x0F, inst, labels, bytes)?,
        
        InstructionKind::Add => encode_arithmetic(0x10, inst, labels, bytes)?,
        InstructionKind::Sub => encode_arithmetic(0x11, inst, labels, bytes)?,
        InstructionKind::Cmp => encode_arithmetic(0x12, inst, labels, bytes)?,
        InstructionKind::And => encode_arithmetic(0x13, inst, labels, bytes)?,
        InstructionKind::Or => encode_arithmetic(0x14, inst, labels, bytes)?,
        InstructionKind::Xor => encode_arithmetic(0x15, inst, labels, bytes)?,
        InstructionKind::Mul => encode_arithmetic(0x16, inst, labels, bytes)?,
        InstructionKind::Div => encode_arithmetic(0x17, inst, labels, bytes)?,
        
        InstructionKind::Mov => encode_mov(inst, bytes)?,
        InstructionKind::Inc => encode_single_reg(0x18, inst, bytes)?,
        InstructionKind::Dec => encode_single_reg(0x19, inst, bytes)?,
        
        InstructionKind::Bit => encode_arithmetic(0x1A, inst, labels, bytes)?,
        InstructionKind::Bset => encode_arithmetic(0x1B, inst, labels, bytes)?,
        InstructionKind::Bclr => encode_arithmetic(0x1C, inst, labels, bytes)?,
        
        InstructionKind::Cop => {
            bytes.push(0x44);
            if let Some(Operand::Single(AddressingMode::Immediate(val))) = &inst.operand {
                bytes.push(*val as u8);
            } else {
                return Err(AsmError::MissingOperand {
                    line: inst.line,
                    instruction: "COP".to_string(),
                });
            }
        }
        
        InstructionKind::Jmp => encode_jump(0x20, inst, labels, bytes)?,
        InstructionKind::Jsr => encode_jump(0x21, inst, labels, bytes)?,
        
        InstructionKind::Bra => encode_branch(0x30, inst, labels, bytes)?,
        InstructionKind::Beq => encode_branch(0x31, inst, labels, bytes)?,
        InstructionKind::Bne => encode_branch(0x32, inst, labels, bytes)?,
        InstructionKind::Bcs => encode_branch(0x33, inst, labels, bytes)?,
        InstructionKind::Bcc => encode_branch(0x34, inst, labels, bytes)?,
        InstructionKind::Bmi => encode_branch(0x35, inst, labels, bytes)?,
        InstructionKind::Bpl => encode_branch(0x36, inst, labels, bytes)?,
        InstructionKind::Bvs => encode_branch(0x37, inst, labels, bytes)?,
        InstructionKind::Bvc => encode_branch(0x38, inst, labels, bytes)?,
    }
    Ok(())
}

fn encode_load_store(
    imm_opcode: u8,
    abs_opcode: u8,
    ind_opcode: u8,
    inst: &RawInstruction,
    labels: &HashMap<String, u32>,
    bytes: &mut Vec<u8>,
) -> Result<(), AsmError> {
    match &inst.operand {
        Some(Operand::Single(AddressingMode::Immediate(val))) => {
            bytes.push(imm_opcode);
            bytes.extend_from_slice(&(*val as u16).to_le_bytes());
        }
        Some(Operand::Single(AddressingMode::Absolute(addr))) => {
            bytes.push(abs_opcode);
            bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
        }
        Some(Operand::Label(name)) => {
            let addr = labels.get(name).ok_or(AsmError::LabelNotFound {
                name: name.clone(),
            })?;
            bytes.push(abs_opcode);
            bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
        }
        Some(Operand::Single(AddressingMode::Indirect)) => {
            bytes.push(ind_opcode);
        }
        Some(Operand::Single(AddressingMode::IndirectOffset(offset))) => {
            bytes.push(ind_opcode + 1); // Assume offset variant
            bytes.extend_from_slice(&(*offset as i16).to_le_bytes());
        }
        _ => {
            return Err(AsmError::MissingOperand {
                line: inst.line,
                instruction: format!("{:?}", inst.kind),
            });
        }
    }
    Ok(())
}

fn encode_arithmetic(
    opcode: u8,
    inst: &RawInstruction,
    _labels: &HashMap<String, u32>,
    bytes: &mut Vec<u8>,
) -> Result<(), AsmError> {
    match &inst.operand {
        Some(Operand::Single(AddressingMode::Immediate(val))) => {
            bytes.push(opcode);
            bytes.extend_from_slice(&(*val as u16).to_le_bytes());
        }
        Some(Operand::Single(AddressingMode::Register(reg))) => {
            bytes.push(opcode);
            bytes.push(*reg);
        }
        _ => {
            return Err(AsmError::MissingOperand {
                line: inst.line,
                instruction: format!("{:?}", inst.kind),
            });
        }
    }
    Ok(())
}

fn encode_mov(inst: &RawInstruction, bytes: &mut Vec<u8>) -> Result<(), AsmError> {
    bytes.push(0x17);
    match &inst.operand {
        Some(Operand::Dual(src, dst)) => {
            match src {
                AddressingMode::Register(src_reg) => bytes.push(*src_reg),
                _ => return Err(AsmError::InvalidAddressingMode {
                    line: inst.line,
                    operand: "src".to_string(),
                }),
            }
            match dst {
                AddressingMode::Register(dst_reg) => bytes.push(*dst_reg),
                _ => return Err(AsmError::InvalidAddressingMode {
                    line: inst.line,
                    operand: "dst".to_string(),
                }),
            }
        }
        _ => {
            return Err(AsmError::MissingOperand {
                line: inst.line,
                instruction: "MOV".to_string(),
            });
        }
    }
    Ok(())
}

fn encode_single_reg(opcode: u8, inst: &RawInstruction, bytes: &mut Vec<u8>) -> Result<(), AsmError> {
    bytes.push(opcode);
    match &inst.operand {
        Some(Operand::Single(AddressingMode::Register(reg))) => {
            bytes.push(*reg);
        }
        _ => {
            return Err(AsmError::MissingOperand {
                line: inst.line,
                instruction: format!("{:?}", inst.kind),
            });
        }
    }
    Ok(())
}

fn encode_jump(
    opcode: u8,
    inst: &RawInstruction,
    labels: &HashMap<String, u32>,
    bytes: &mut Vec<u8>,
) -> Result<(), AsmError> {
    bytes.push(opcode);
    let addr = match &inst.operand {
        Some(Operand::Single(AddressingMode::Absolute(addr))) => *addr,
        Some(Operand::Label(name)) => *labels.get(name).ok_or(AsmError::LabelNotFound {
            name: name.clone(),
        })?,
        _ => {
            return Err(AsmError::MissingOperand {
                line: inst.line,
                instruction: format!("{:?}", inst.kind),
            });
        }
    };
    bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
    Ok(())
}

fn encode_branch(
    opcode: u8,
    inst: &RawInstruction,
    labels: &HashMap<String, u32>,
    bytes: &mut Vec<u8>,
) -> Result<(), AsmError> {
    bytes.push(opcode);
    let target = match &inst.operand {
        Some(Operand::Single(AddressingMode::Absolute(addr))) => *addr,
        Some(Operand::Label(name)) => *labels.get(name).ok_or(AsmError::LabelNotFound {
            name: name.clone(),
        })?,
        _ => {
            return Err(AsmError::MissingOperand {
                line: inst.line,
                instruction: format!("{:?}", inst.kind),
            });
        }
    };
    let pc_after_operand = inst.address + instruction_length(&inst.kind, &inst.operand);
    let offset = target as i32 - pc_after_operand as i32;
    if offset < -128 || offset > 127 {
        return Err(AsmError::BranchOutOfRange {
            label: "".to_string(), // Simplified
            offset,
        });
    }
    bytes.push(offset as u8);
    Ok(())
}

/// Generate the BIOS image by assembling NRAW sources and laying out at correct offsets in 64 KiB.
pub fn generate_bios() -> Result<Vec<u8>, AsmError> {
    let bios_source = r#"
; Interrupt handlers
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

; BIOS initialization
bios_init:
    CLI
    LDA #0x0001
    STA 0x100000  ; DISPCTL
    LDA 0x400000  ; Check cartridge
    BEQ no_cart
    JMP 0x400000  ; Boot cartridge
no_cart:
    WFI
    BRA no_cart

; Syscall entry (at 0x0100)
syscall_entry:
    CMP #0
    BEQ syscall_get_version
    CMP #1
    BEQ syscall_vblank_wait
    CMP #2
    BEQ syscall_delay
    LDA #0xFFFF  ; Error
    RTS
syscall_get_version:
    LDA #0x0100  ; Version 1.0
    RTS
syscall_vblank_wait:
    LDA 0x100002  ; DISPSTAT
    AND #0x0001   ; VBlank flag
    BEQ syscall_vblank_wait
    RTS
syscall_delay:
    MOV R0, A
delay_loop:
    DEC A
    BNE delay_loop
    RTS
"#;
    let program = assemble(bios_source)?;
    let mut bios = vec![0xFF; 0x10000]; // 64 KiB filled with HLT
    
    // Set interrupt vectors (24-bit addresses)
    let reset_addr = program.labels["bios_init"];
    bios[0..3].copy_from_slice(&reset_addr.to_le_bytes()[..3]);
    
    let swi_addr = program.labels["swi_handler"];
    bios[3..6].copy_from_slice(&swi_addr.to_le_bytes()[..3]);
    
    let pad_event_addr = program.labels["pad_event_handler"];
    bios[6..9].copy_from_slice(&pad_event_addr.to_le_bytes()[..3]);
    
    let timer0_addr = program.labels["timer0_handler"];
    bios[9..12].copy_from_slice(&timer0_addr.to_le_bytes()[..3]);
    
    let apu_buf_empty_addr = program.labels["apu_buf_empty_handler"];
    bios[12..15].copy_from_slice(&apu_buf_empty_addr.to_le_bytes()[..3]);
    
    let vlu_done_addr = program.labels["vlu_done_handler"];
    bios[15..18].copy_from_slice(&vlu_done_addr.to_le_bytes()[..3]);
    
    let dma_done_addr = program.labels["dma_done_handler"];
    bios[18..21].copy_from_slice(&dma_done_addr.to_le_bytes()[..3]);
    
    let hblank_addr = program.labels["hblank_handler"];
    bios[21..24].copy_from_slice(&hblank_addr.to_le_bytes()[..3]);
    
    let nmi_addr = program.labels["nmi_handler"];
    bios[24..27].copy_from_slice(&nmi_addr.to_le_bytes()[..3]);
    
    // Copy the assembled code
    bios[0..program.bytes.len()].copy_from_slice(&program.bytes);
    
    Ok(bios)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_simple_program() {
        let source = r#"
start:
    LDA #0x1234
    STA data
    BRA start

data:
    NOP
"#;
        let program = assemble(source).expect("assemble");
        assert_eq!(program.labels.get("start"), Some(&0));
        assert_eq!(program.labels.get("data"), Some(&9));
        assert_eq!(
            program.bytes,
            vec![0x01, 0x34, 0x12, 0x02, 0x09, 0x00, 0x00, 0x30, 0xF7, 0x00,]
        );
    }

    #[test]
    fn branch_out_of_range_error() {
        let mut source = String::from("start:\n    BRA far\n");
        for _ in 0..130 {
            source.push_str("    NOP\n");
        }
        source.push_str("far:\n    NOP\n");
        let result = assemble(&source);
        assert!(matches!(result, Err(AsmError::BranchOutOfRange { .. })));
    }
    
    #[test]
    fn assembles_mov_instruction() {
        let source = "MOV R0, A";
        let program = assemble(source).expect("assemble MOV");
        assert_eq!(program.bytes, vec![0x17, 4, 0]); // MOV opcode, R0(4), A(0)
    }
    
    #[test]
    fn assembles_inc_dec() {
        let source = r#"
    INC A
    DEC R0
"#;
        let program = assemble(source).expect("assemble INC/DEC");
        assert_eq!(program.bytes, vec![0x18, 0, 0x19, 4]);
    }

    #[test]
    fn generates_bios() {
        let bios = generate_bios().expect("generate BIOS");
        assert_eq!(bios.len(), 0x10000);
        // Check reset vector points to bios_init
        let bios_init_addr = bios[0] as u32 | ((bios[1] as u32) << 8) | ((bios[2] as u32) << 16);
        assert!(bios_init_addr >= 8); // bios_init should be after the RTI handlers
    }
}
