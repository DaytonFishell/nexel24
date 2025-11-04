//! VLU-24 vector coprocessor stub

pub struct Vlu {
    // Simple placeholder for vector registers (e.g., 8 vectors of 3 components)
    vectors: [[i16; 3]; 8],
}

impl Vlu {
    pub fn new() -> Self {
        Self {
            vectors: [[0; 3]; 8],
        }
    }

    /// Perform a dummy vector operation and signal completion interrupt
    pub fn compute(&mut self, cpu: &mut crate::cpu::Cpu) {
        // Dummy operation: increment first component of first vector
        self.vectors[0][0] = self.vectors[0][0].wrapping_add(1);
        // Signal VLU_DONE interrupt (int id 4)
        cpu.request_interrupt(4);
    }
}
