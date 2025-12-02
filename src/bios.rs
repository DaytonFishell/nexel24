use crate::nraw::assemble;

const BIOS_SIZE: usize = 0x10000;
const BIOS_VECTOR_COUNT: usize = 8;
const BIOS_CODE_OFFSET: usize = 0x20;

const BIOS_SOURCE: &str = r#"
start:
    SEI
loop:
    BRA loop
"#;

/// Produce the default BIOS image used by the emulator.
pub fn default_bios() -> Vec<u8> {
    let program = assemble(BIOS_SOURCE).expect("invalid BIOS source");
    let mut bios = vec![0xFF; BIOS_SIZE];
    let entry =
        0xFF0000 + BIOS_CODE_OFFSET as u32 + program.labels.get("start").copied().unwrap_or(0);

    for idx in 0..BIOS_VECTOR_COUNT {
        let offset = idx * 3;
        bios[offset] = (entry & 0xFF) as u8;
        bios[offset + 1] = ((entry >> 8) & 0xFF) as u8;
        bios[offset + 2] = ((entry >> 16) & 0xFF) as u8;
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
    fn vectors_point_to_start() {
        let bios = default_bios();
        let program = assemble(BIOS_SOURCE).expect("assemble BIOS source");
        let start_offset = program.labels.get("start").copied().unwrap_or(0);
        let entry = 0xFF0000 + BIOS_CODE_OFFSET as u32 + start_offset;
        for idx in 0..BIOS_VECTOR_COUNT {
            let offset = idx * 3;
            assert_eq!(bios[offset], (entry & 0xFF) as u8);
            assert_eq!(bios[offset + 1], ((entry >> 8) & 0xFF) as u8);
            assert_eq!(bios[offset + 2], ((entry >> 16) & 0xFF) as u8);
        }
    }
}
