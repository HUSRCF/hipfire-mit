// SPDX-License-Identifier: MIT
//! rdna-compute: Kernel compilation, caching, and dispatch for RDNA GPUs.

mod compiler;
mod dispatch;
mod kernels;
pub mod pool;
pub mod profile;
pub mod profiler;

pub use compiler::{KernelArtifactDiagnostic, KernelArtifactOrigin, KernelCompiler};
pub use dispatch::{
    gemv_dp4a_enabled, has_wmma_f16, minimum_hip_version, resolve_target_arch, DType, Gpu,
    GpuTensor,
};
pub use kernels::GEMV_SRC;
