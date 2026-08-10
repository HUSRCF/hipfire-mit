// SPDX-License-Identifier: MIT
//! Compile one small kernel and print its cache/provenance diagnostic record.

use rdna_compute::{Gpu, GEMV_SRC};

fn main() {
    let mut gpu = Gpu::init().expect("GPU init");
    let module = "diagnostic_gemv";
    gpu.ensure_kernel_public(module, GEMV_SRC, "gemv_f32")
        .expect("prepare diagnostic kernel");

    let records = gpu.kernel_artifact_diagnostics();
    let record = records
        .iter()
        .find(|record| record.module == module)
        .expect("diagnostic record");
    println!("module={}", record.module);
    println!("arch={}", record.arch);
    println!("source_arch_hash={}", record.source_arch_hash);
    println!("origin={:?}", record.origin);
    println!("validated={}", record.validated());
    println!("artifact={}", record.path.display());
}
