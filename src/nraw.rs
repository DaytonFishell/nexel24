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
    And,
    Or,
    Xor,
    Jmp,
    Jsr,
    Rts,
    Bra,
    Beq,
    Bne,
    Sei,
    Cli,
    Rti,
    Hlt,
}

enum Operand {
    Value(u32),
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
    let mut instructions = Vec::new();
    let mut address = 0u32;

    for (line_idx, line) in source.lines().enumerate() {
        let stripped = line.split(';').next().unwrap_or("").trim();
        if stripped.is_empty() {
            continue;
        }

        let mut working = stripped;
        loop {
            if let Some(colon) = working.find(':') {
                let label = working[..colon].trim();
                if !label.is_empty() {
                    if labels.contains_key(label) {
                        return Err(AsmError::DuplicateLabel {
                            line: line_idx + 1,
                            name: label.to_string(),
                        });
                    }
                    labels.insert(label.to_string(), address);
                }
                working = working[colon + 1..].trim();
                if working.is_empty() {
                    break;
                }
                continue;
            }
            break;
        }

        if working.is_empty() {
            continue;
        }

        let mut parts = working.split_whitespace();
        let op = parts.next().unwrap();
        let name = op.to_uppercase();
        let operand_text = parts.next();
        let extra = parts.next();
        if extra.is_some() {
            return Err(AsmError::UnexpectedOperand {
                line: line_idx + 1,
                instruction: name.clone(),
            });
        }

        let kind = match name.as_str() {
            "NOP" => InstructionKind::Nop,
            "LDA" => InstructionKind::Lda,
            "STA" => InstructionKind::Sta,
            "LDX" => InstructionKind::Ldx,
            "STX" => InstructionKind::Stx,
            "LDY" => InstructionKind::Ldy,
            "STY" => InstructionKind::Sty,
            "ADD" => InstructionKind::Add,
            "SUB" => InstructionKind::Sub,
            "AND" => InstructionKind::And,
            "OR" => InstructionKind::Or,
            "XOR" => InstructionKind::Xor,
            "JMP" => InstructionKind::Jmp,
            "JSR" => InstructionKind::Jsr,
            "RTS" => InstructionKind::Rts,
            "BRA" => InstructionKind::Bra,
            "BEQ" => InstructionKind::Beq,
            "BNE" => InstructionKind::Bne,
            "SEI" => InstructionKind::Sei,
            "CLI" => InstructionKind::Cli,
            "RTI" => InstructionKind::Rti,
            "HLT" => InstructionKind::Hlt,
            _ => {
                return Err(AsmError::UnknownInstruction {
                    line: line_idx + 1,
                    token: op.to_string(),
                });
            }
        };

        let operand = match kind {
            InstructionKind::Nop
            | InstructionKind::Rts
            | InstructionKind::Sei
            | InstructionKind::Cli
            | InstructionKind::Rti
            | InstructionKind::Hlt => {
                if operand_text.is_some() {
                    return Err(AsmError::UnexpectedOperand {
                        line: line_idx + 1,
                        instruction: name.clone(),
                    });
                }
                None
            }
            InstructionKind::Lda
            | InstructionKind::Ldx
            | InstructionKind::Ldy
            | InstructionKind::Add
            | InstructionKind::Sub
            | InstructionKind::And
            | InstructionKind::Or
            | InstructionKind::Xor => {
                let operand_text = operand_text.ok_or(AsmError::MissingOperand {
                    line: line_idx + 1,
                    instruction: name.clone(),
                })?;
                if !operand_text.starts_with('#') {
                    return Err(AsmError::InvalidNumber {
                        line: line_idx + 1,
                        operand: operand_text.to_string(),
                    });
                }
                let raw = operand_text[1..].trim();
                Some(Operand::Value(parse_number(raw, line_idx + 1)?))
            }
            InstructionKind::Sta
            | InstructionKind::Stx
            | InstructionKind::Sty
            | InstructionKind::Jmp
            | InstructionKind::Jsr => {
                let operand_text = operand_text.ok_or(AsmError::MissingOperand {
                    line: line_idx + 1,
                    instruction: name.clone(),
                })?;
                if let Ok(value) = parse_number(operand_text, line_idx + 1) {
                    Some(Operand::Value(value))
                } else {
                    Some(Operand::Label(operand_text.to_string()))
                }
            }
            InstructionKind::Bra | InstructionKind::Beq | InstructionKind::Bne => {
                let operand_text = operand_text.ok_or(AsmError::MissingOperand {
                    line: line_idx + 1,
                    instruction: name.clone(),
                })?;
                if let Ok(value) = parse_number(operand_text, line_idx + 1) {
                    Some(Operand::Value(value))
                } else {
                    Some(Operand::Label(operand_text.to_string()))
                }
            }
        };

        let inst_length = instruction_length(&kind);
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
        match inst.kind {
            InstructionKind::Nop => {
                bytes.push(0x00);
            }
            InstructionKind::Hlt => {
                bytes.push(0xFF);
            }
            InstructionKind::Rts => {
                bytes.push(0x22);
            }
            InstructionKind::Sei => {
                bytes.push(0x40);
            }
            InstructionKind::Cli => {
                bytes.push(0x41);
            }
            InstructionKind::Rti => {
                bytes.push(0x42);
            }
            InstructionKind::Lda => {
                bytes.push(0x01);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::Ldx => {
                bytes.push(0x03);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::Ldy => {
                bytes.push(0x05);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::Add => {
                bytes.push(0x10);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::Sub => {
                bytes.push(0x11);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::And => {
                bytes.push(0x12);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::Or => {
                bytes.push(0x13);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::Xor => {
                bytes.push(0x14);
                let value = operand_value(&inst, &labels)? as u16;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            InstructionKind::Sta => {
                bytes.push(0x02);
                let addr = operand_address(&inst, &labels)?;
                bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
            }
            InstructionKind::Stx => {
                bytes.push(0x04);
                let addr = operand_address(&inst, &labels)?;
                bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
            }
            InstructionKind::Sty => {
                bytes.push(0x06);
                let addr = operand_address(&inst, &labels)?;
                bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
            }
            InstructionKind::Jmp => {
                bytes.push(0x20);
                let addr = operand_address(&inst, &labels)?;
                bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
            }
            InstructionKind::Jsr => {
                bytes.push(0x21);
                let addr = operand_address(&inst, &labels)?;
                bytes.extend_from_slice(&addr.to_le_bytes()[..3]);
            }
            InstructionKind::Bra | InstructionKind::Beq | InstructionKind::Bne => {
                let opcode = match inst.kind {
                    InstructionKind::Bra => 0x30,
                    InstructionKind::Beq => 0x31,
                    InstructionKind::Bne => 0x32,
                    _ => unreachable!(),
                };
                bytes.push(opcode);
                let offset = branch_offset(&inst, &labels)?;
                bytes.push(offset as u8);
            }
        }
    }

    Ok(AssembledProgram { bytes, labels })
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

fn instruction_length(kind: &InstructionKind) -> u32 {
    match kind {
        InstructionKind::Nop
        | InstructionKind::Rts
        | InstructionKind::Sei
        | InstructionKind::Cli
        | InstructionKind::Rti
        | InstructionKind::Hlt => 1,
        InstructionKind::Lda
        | InstructionKind::Ldx
        | InstructionKind::Ldy
        | InstructionKind::Add
        | InstructionKind::Sub
        | InstructionKind::And
        | InstructionKind::Or
        | InstructionKind::Xor
        | InstructionKind::Bra
        | InstructionKind::Beq
        | InstructionKind::Bne => 2,
        InstructionKind::Sta
        | InstructionKind::Stx
        | InstructionKind::Sty
        | InstructionKind::Jmp
        | InstructionKind::Jsr => 4,
    }
}

fn operand_value(inst: &RawInstruction, labels: &HashMap<String, u32>) -> Result<u32, AsmError> {
    match inst.operand {
        Some(Operand::Value(v)) => Ok(v),
        Some(Operand::Label(ref lbl)) => labels
            .get(lbl)
            .copied()
            .ok_or(AsmError::LabelNotFound { name: lbl.clone() }),
        None => Err(AsmError::MissingOperand {
            line: inst.line,
            instruction: format!("{:?}", inst.kind),
        }),
    }
}

fn operand_address(inst: &RawInstruction, labels: &HashMap<String, u32>) -> Result<u32, AsmError> {
    let value = operand_value(inst, labels)?;
    if value >= (1 << 24) {
        return Err(AsmError::InvalidNumber {
            line: inst.line,
            operand: value.to_string(),
        });
    }
    Ok(value)
}

fn branch_offset(inst: &RawInstruction, labels: &HashMap<String, u32>) -> Result<i8, AsmError> {
    let target = operand_value(inst, labels)?;
    let pc_after_operand = inst.address + instruction_length(&inst.kind);
    let offset = target as i32 - pc_after_operand as i32;
    if offset < -128 || offset > 127 {
        return Err(AsmError::BranchOutOfRange {
            label: match inst.operand {
                Some(Operand::Label(ref name)) => name.clone(),
                _ => format!("0x{:02X}", target),
            },
            offset,
        });
    }
    Ok(offset as i8)
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
        let source = r#"
start:
    BRA far
"#;
        let result = assemble(source);
        assert!(matches!(result, Err(AsmError::BranchOutOfRange { .. })));
    }
}
