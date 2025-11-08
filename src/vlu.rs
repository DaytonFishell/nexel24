//! VLU-24 vector coprocessor implementation.
//!
//! The VLU exposes eight vector registers and four matrix registers.  All
//! operations are 3D and operate on 32-bit floating point data which mirrors the
//! behaviour of the original hardware's 24-bit fixed point units.  The
//! implementation favours determinism and correctness over raw throughput.
//!
//! Each invocation of [`Vlu::compute`] performs a single vector job and then
//! raises the `VLU_DONE` interrupt (interrupt id 4).  Callers can load registers
//! via [`set_vector`] and [`set_matrix`] prior to scheduling jobs, and then
//! inspect the results using [`vector`], [`scalar_result`] or the returned
//! [`VluResult`].

use std::fmt;

use thiserror::Error;

const VECTOR_REGISTER_COUNT: usize = 8;
const MATRIX_REGISTER_COUNT: usize = 4;

/// An individual 3D vector used by the VLU.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn from_array(value: [f32; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }

    fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    fn dot(self, rhs: Self) -> f32 {
        self.x.mul_add(rhs.x, self.y.mul_add(rhs.y, self.z * rhs.z))
    }

    fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    fn normalize(self) -> Self {
        let magnitude_sq = self.dot(self);
        if magnitude_sq <= f32::EPSILON {
            return Self::default();
        }

        #[cfg(feature = "fast-math")]
        let inv_len = fast_inv_sqrt(magnitude_sq);

        #[cfg(not(feature = "fast-math"))]
        let inv_len = 1.0 / magnitude_sq.sqrt();

        Self {
            x: self.x * inv_len,
            y: self.y * inv_len,
            z: self.z * inv_len,
        }
    }
}

/// 3×3 matrix register used for affine transforms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Mat3 {
    rows: [Vec3; 3],
}

impl Mat3 {
    fn from_array(value: [[f32; 3]; 3]) -> Self {
        Self {
            rows: [
                Vec3::from_array(value[0]),
                Vec3::from_array(value[1]),
                Vec3::from_array(value[2]),
            ],
        }
    }

    fn to_array(self) -> [[f32; 3]; 3] {
        [
            self.rows[0].to_array(),
            self.rows[1].to_array(),
            self.rows[2].to_array(),
        ]
    }

    fn mul_vec(self, vec: Vec3) -> Vec3 {
        Vec3::new(self.rows[0].dot(vec), self.rows[1].dot(vec), self.rows[2].dot(vec))
    }
}

/// Job description supplied to the VLU.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VluJob {
    /// Apply matrix `matrix` to vector `vec` and write result into `dest`.
    Transform {
        dest: usize,
        vec: usize,
        matrix: usize,
    },
    /// Compute the dot product of vectors `a` and `b`.
    Dot { a: usize, b: usize },
    /// Compute the cross product of `a` × `b` storing into `dest`.
    Cross {
        dest: usize,
        a: usize,
        b: usize,
    },
    /// Normalise vector `src` and write it into `dest`.
    Normalize { dest: usize, src: usize },
}

/// Result of a VLU computation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VluResult {
    Vector([f32; 3]),
    Scalar(f32),
}

impl fmt::Display for VluResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vector(v) => write!(f, "[{:.6}, {:.6}, {:.6}]", v[0], v[1], v[2]),
            Self::Scalar(s) => write!(f, "{:.6}", s),
        }
    }
}

/// VLU specific errors.
#[derive(Debug, Error, PartialEq)]
pub enum VluError {
    #[error("invalid vector register {0}")]
    InvalidVectorRegister(usize),
    #[error("invalid matrix register {0}")]
    InvalidMatrixRegister(usize),
}

/// VLU-24 vector coprocessor.
pub struct Vlu {
    vectors: [Vec3; VECTOR_REGISTER_COUNT],
    matrices: [Mat3; MATRIX_REGISTER_COUNT],
    last_scalar: f32,
}

impl Vlu {
    /// Create a new VLU instance with zeroed registers.
    pub fn new() -> Self {
        Self {
            vectors: [Vec3::default(); VECTOR_REGISTER_COUNT],
            matrices: [Mat3::default(); MATRIX_REGISTER_COUNT],
            last_scalar: 0.0,
        }
    }

    /// Load a vector register.
    pub fn set_vector(&mut self, index: usize, value: [f32; 3]) -> Result<(), VluError> {
        let slot = self
            .vectors
            .get_mut(index)
            .ok_or(VluError::InvalidVectorRegister(index))?;
        *slot = Vec3::from_array(value);
        Ok(())
    }

    /// Read a vector register.
    pub fn vector(&self, index: usize) -> Result<[f32; 3], VluError> {
        self.vectors
            .get(index)
            .copied()
            .ok_or(VluError::InvalidVectorRegister(index))
            .map(Vec3::to_array)
    }

    /// Load a matrix register.
    pub fn set_matrix(&mut self, index: usize, value: [[f32; 3]; 3]) -> Result<(), VluError> {
        let slot = self
            .matrices
            .get_mut(index)
            .ok_or(VluError::InvalidMatrixRegister(index))?;
        *slot = Mat3::from_array(value);
        Ok(())
    }

    /// Read a matrix register.
    pub fn matrix(&self, index: usize) -> Result<[[f32; 3]; 3], VluError> {
        self.matrices
            .get(index)
            .copied()
            .ok_or(VluError::InvalidMatrixRegister(index))
            .map(Mat3::to_array)
    }

    /// Last scalar result produced by [`VluJob::Dot`].
    pub fn scalar_result(&self) -> f32 {
        self.last_scalar
    }

    /// Perform a vector job and raise the VLU completion interrupt.
    pub fn compute(
        &mut self,
        cpu: &mut crate::cpu::Cpu,
        job: VluJob,
    ) -> Result<VluResult, VluError> {
        let result = match job {
            VluJob::Transform { dest, vec, matrix } => {
                let vec = *self
                    .vectors
                    .get(vec)
                    .ok_or(VluError::InvalidVectorRegister(vec))?;
                let mat = *self
                    .matrices
                    .get(matrix)
                    .ok_or(VluError::InvalidMatrixRegister(matrix))?;
                let transformed = mat.mul_vec(vec);
                *self
                    .vectors
                    .get_mut(dest)
                    .ok_or(VluError::InvalidVectorRegister(dest))? = transformed;
                VluResult::Vector(transformed.to_array())
            }
            VluJob::Dot { a, b } => {
                let lhs = *self
                    .vectors
                    .get(a)
                    .ok_or(VluError::InvalidVectorRegister(a))?;
                let rhs = *self
                    .vectors
                    .get(b)
                    .ok_or(VluError::InvalidVectorRegister(b))?;
                let dot = lhs.dot(rhs);
                self.last_scalar = dot;
                VluResult::Scalar(dot)
            }
            VluJob::Cross { dest, a, b } => {
                let lhs = *self
                    .vectors
                    .get(a)
                    .ok_or(VluError::InvalidVectorRegister(a))?;
                let rhs = *self
                    .vectors
                    .get(b)
                    .ok_or(VluError::InvalidVectorRegister(b))?;
                let cross = lhs.cross(rhs);
                *self
                    .vectors
                    .get_mut(dest)
                    .ok_or(VluError::InvalidVectorRegister(dest))? = cross;
                VluResult::Vector(cross.to_array())
            }
            VluJob::Normalize { dest, src } => {
                let vec = *self
                    .vectors
                    .get(src)
                    .ok_or(VluError::InvalidVectorRegister(src))?;
                let normalized = vec.normalize();
                *self
                    .vectors
                    .get_mut(dest)
                    .ok_or(VluError::InvalidVectorRegister(dest))? = normalized;
                VluResult::Vector(normalized.to_array())
            }
        };

        cpu.request_interrupt(4);

        Ok(result)
    }
}

#[cfg(feature = "fast-math")]
fn fast_inv_sqrt(value: f32) -> f32 {
    // Quake III style fast inverse square root, tweaked for Rust's strict aliasing.
    let x2 = value * 0.5;
    let mut y = value;
    let mut i = y.to_bits();
    i = 0x5f3759df - (i >> 1);
    y = f32::from_bits(i);
    y * (1.5 - x2 * y * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cpu() -> crate::cpu::Cpu {
        crate::cpu::Cpu::new()
    }

    #[test]
    fn transform_applies_matrix() {
        let mut vlu = Vlu::new();
        let mut cpu = cpu();

        vlu.set_vector(0, [1.0, 2.0, 3.0]).unwrap();
        vlu.set_matrix(0, [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]])
            .unwrap();

        let result = vlu
            .compute(
                &mut cpu,
                VluJob::Transform {
                    dest: 1,
                    vec: 0,
                    matrix: 0,
                },
            )
            .unwrap();

        assert_eq!(result, VluResult::Vector([1.0, 4.0, 9.0]));
        assert_eq!(vlu.vector(1).unwrap(), [1.0, 4.0, 9.0]);
    }

    #[test]
    fn dot_product_returns_scalar() {
        let mut vlu = Vlu::new();
        let mut cpu = cpu();
        vlu.set_vector(0, [1.0, 3.0, -5.0]).unwrap();
        vlu.set_vector(1, [4.0, -2.0, -1.0]).unwrap();

        let result = vlu
            .compute(&mut cpu, VluJob::Dot { a: 0, b: 1 })
            .unwrap();

        assert_eq!(result, VluResult::Scalar(3.0));
        assert_eq!(vlu.scalar_result(), 3.0);
    }

    #[test]
    fn cross_product_stores_vector() {
        let mut vlu = Vlu::new();
        let mut cpu = cpu();
        vlu.set_vector(0, [1.0, 0.0, 0.0]).unwrap();
        vlu.set_vector(1, [0.0, 1.0, 0.0]).unwrap();

        let result = vlu
            .compute(
                &mut cpu,
                VluJob::Cross {
                    dest: 2,
                    a: 0,
                    b: 1,
                },
            )
            .unwrap();

        assert_eq!(result, VluResult::Vector([0.0, 0.0, 1.0]));
        assert_eq!(vlu.vector(2).unwrap(), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn normalize_handles_zero_vector() {
        let mut vlu = Vlu::new();
        let mut cpu = cpu();
        vlu.set_vector(0, [0.0, 0.0, 0.0]).unwrap();

        let result = vlu
            .compute(
                &mut cpu,
                VluJob::Normalize { dest: 1, src: 0 },
            )
            .unwrap();

        assert_eq!(result, VluResult::Vector([0.0, 0.0, 0.0]));
    }

    #[test]
    fn invalid_register_returns_error() {
        let mut vlu = Vlu::new();
        let mut cpu = cpu();
        vlu.set_vector(0, [1.0, 0.0, 0.0]).unwrap();

        let err = vlu
            .compute(
                &mut cpu,
                VluJob::Transform {
                    dest: 8,
                    vec: 0,
                    matrix: 0,
                },
            )
            .unwrap_err();

        assert_eq!(err, VluError::InvalidVectorRegister(8));
    }
}
