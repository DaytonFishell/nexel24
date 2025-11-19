# VLU-24 Vector Coprocessor

The VLU-24 provides sixteen 3D vector registers and four 3×3 matrix registers
that can be combined to accelerate linear algebra workloads such as rotation,
lighting and physics calculations.

## Registers

| Register | Description |
|----------|-------------|
| V0-V15   | 3-component vector registers stored as 32-bit floating point values |
| M0-M3    | 3×3 matrix registers stored in row-major order |
| SCALAR   | Last scalar result (updated by dot products) |

## Operations

Operations are submitted through [`Vlu::compute`](../src/vlu.rs) by supplying a
[`VluJob`]. Each job completes synchronously, updates the relevant registers, and
raises the `VLU_DONE` interrupt (`id = 4`).

### Transform

Applies a matrix to a vector and writes the result into a destination vector.

```rust
vlu.compute(
    &mut cpu,
    VluJob::Transform {
        dest: 1,
        vec: 0,
        matrix: 2,
    },
)?;
```

### Dot Product

Computes the dot product between two vectors, writes the result to the scalar
register and returns the scalar.

```rust
let dot = vlu.compute(&mut cpu, VluJob::Dot { a: 0, b: 1 })?;
let last = vlu.scalar_result();
```

### Cross Product

Generates the cross product `a × b` and stores it in the destination vector.

```rust
vlu.compute(
    &mut cpu,
    VluJob::Cross {
        dest: 2,
        a: 0,
        b: 1,
    },
)?;
```

### Normalize

Normalizes the provided vector and writes the unit vector into the destination
register. Zero-length vectors normalize to `[0.0, 0.0, 0.0]`.

```rust
vlu.compute(
    &mut cpu,
    VluJob::Normalize { dest: 3, src: 0 },
)?;
```

## Fast Math Feature

Enabling the `fast-math` Cargo feature switches the normalization routine to use
an approximate inverse square root (Quake III style). This trades a small amount
of precision for higher throughput and mirrors the optional fast path available
on the original hardware.
