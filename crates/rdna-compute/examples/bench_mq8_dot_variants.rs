// SPDX-License-Identifier: MIT
//! MQ8 decode GEMV negative experiment: gfx11 hardware dot4 versus scalar VALU.
//!
//! Both variants are compiled from the current production MQ8 source. The
//! control keeps `sudot4(true, a, true, b, acc, false)`; the candidate
//! replaces each packed dot with four explicit scalar integer multiplies.
//! Outputs must agree before timings are accepted.

use hip_bridge::KernargBlob;
use rdna_compute::{DType, Gpu, GpuTensor};
use std::ffi::c_void;

const MQ8_SOURCE: &str = include_str!("../../../kernels/src/gemv_mq8g256.hip");
const DOT_MODULE: &str = "bench_mq8_dot4_module";
const DOT_KERNEL: &str = "bench_gemv_mq8_dot4";
const SCALAR_MODULE: &str = "bench_mq8_scalar_module";
const SCALAR_KERNEL: &str = "bench_gemv_mq8_scalar";

fn main() {
    let mut gpu = Gpu::init().expect("GPU init");
    if !gpu.arch.starts_with("gfx11") {
        eprintln!(
            "SKIP: MQ8 sudot4 experiment requires gfx11 (arch={})",
            gpu.arch
        );
        return;
    }

    let dot_source = source_variant(DOT_KERNEL, false);
    let scalar_source = source_variant(SCALAR_KERNEL, true);
    gpu.ensure_kernel_public(DOT_MODULE, &dot_source, DOT_KERNEL)
        .expect("compile dot4 control");
    gpu.ensure_kernel_public(SCALAR_MODULE, &scalar_source, SCALAR_KERNEL)
        .expect("compile scalar candidate");

    let dpm_seconds = std::env::var("HIPFIRE_DPM_WARMUP_SECS")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(5.0);
    gpu.dpm_warmup(dpm_seconds).expect("DPM warmup");

    eprintln!("MQ8 dot negative experiment: arch={}", gpu.arch);
    eprintln!("ratio = scalar_us / dot4_us; values above 1 mean scalar is slower");

    let shapes = [
        (512usize, 4096usize, "kv projection"),
        (4096, 4096, "square projection"),
        (11008, 4096, "gate/up projection"),
        (4096, 11008, "down projection"),
    ];
    let mut ratios = Vec::with_capacity(shapes.len());

    for (shape_index, &(m, k, label)) in shapes.iter().enumerate() {
        assert_eq!(k % 256, 0);
        let weights = synthetic_mq8(m, k, 0x49d2_019b_u64 ^ shape_index as u64);
        let x_q8 = synthetic_i8(k, 0x8ac7_135d_u64 ^ k as u64);
        let x_scales: Vec<f32> = (0..k / 256)
            .map(|group| 0.003 + group as f32 * 0.000_001)
            .collect();

        let d_weights = gpu.upload_raw(&weights, &[weights.len()]).unwrap();
        let d_x_q8 = gpu.upload_raw(&x_q8, &[x_q8.len()]).unwrap();
        let d_x_scales = gpu.upload_f32(&x_scales, &[x_scales.len()]).unwrap();
        let d_y_dot = gpu.zeros(&[m], DType::F32).unwrap();
        let d_y_scalar = gpu.zeros(&[m], DType::F32).unwrap();

        let mut dot_args = gemv_args(&d_weights, &d_x_q8, &d_x_scales, &d_y_dot, m, k);
        let mut scalar_args = gemv_args(&d_weights, &d_x_q8, &d_x_scales, &d_y_scalar, m, k);

        for _ in 0..30 {
            launch(&gpu, DOT_KERNEL, &mut dot_args, m);
            launch(&gpu, SCALAR_KERNEL, &mut scalar_args, m);
        }
        gpu.hip.device_synchronize().unwrap();

        let y_dot = gpu.download_f32(&d_y_dot).unwrap();
        let y_scalar = gpu.download_f32(&d_y_scalar).unwrap();
        let (max_abs, bit_exact) = compare(&y_dot, &y_scalar);
        if max_abs > 1e-6 {
            eprintln!(
                "FAIL {label}: dot/scalar mismatch max_abs={max_abs:.6e} bit_exact={bit_exact}/{m}"
            );
            std::process::exit(1);
        }

        // About 8 GiB of weight traffic per timing sample, bounded so both
        // small and large shapes have useful event durations.
        let iters = ((8usize << 30) / weights.len()).clamp(200, 4000);
        let mut dot_samples = Vec::with_capacity(5);
        let mut scalar_samples = Vec::with_capacity(5);
        for round in 0..5 {
            if round % 2 == 0 {
                dot_samples.push(time_kernel(&gpu, DOT_KERNEL, &mut dot_args, m, iters));
                scalar_samples.push(time_kernel(&gpu, SCALAR_KERNEL, &mut scalar_args, m, iters));
            } else {
                scalar_samples.push(time_kernel(&gpu, SCALAR_KERNEL, &mut scalar_args, m, iters));
                dot_samples.push(time_kernel(&gpu, DOT_KERNEL, &mut dot_args, m, iters));
            }
        }

        let dot_us = median(&mut dot_samples);
        let scalar_us = median(&mut scalar_samples);
        let ratio = scalar_us / dot_us;
        ratios.push(ratio);
        let bytes_per_call = weights.len() + x_q8.len() + x_scales.len() * 4 + m * 4;
        let dot_gbps = bytes_per_call as f64 / (dot_us * 1e-6) / 1e9;
        let scalar_gbps = bytes_per_call as f64 / (scalar_us * 1e-6) / 1e9;
        eprintln!(
            "{label:20} M={m:5} K={k:5} iters={iters:4}  dot4={dot_us:8.3} us ({dot_gbps:6.1} effective GB/s)  scalar={scalar_us:8.3} us ({scalar_gbps:6.1} effective GB/s)  ratio={ratio:.3}  max_abs={max_abs:.1e} bit_exact={bit_exact}/{m}"
        );
    }

    let aggregate = median(&mut ratios);
    let verdict = if aggregate >= 1.05 {
        "REJECT_SCALAR"
    } else if aggregate >= 0.98 {
        "NO_SCALAR_ADVANTAGE"
    } else {
        "SCALAR_CANDIDATE_REQUIRES_FRESH_PROCESS_CONFIRMATION"
    };
    eprintln!("aggregate_median_ratio={aggregate:.3} verdict={verdict}");
}

fn source_variant(kernel_name: &str, scalar: bool) -> String {
    const ENTRY: &str = "extern \"C\" __global__ void gemv_mq8g256(";
    let replacement = format!("extern \"C\" __global__ void {kernel_name}(");
    let mut source = MQ8_SOURCE.replacen(ENTRY, &replacement, 1);
    assert_ne!(source, MQ8_SOURCE, "MQ8 entry marker changed");

    if scalar {
        const INCLUDE: &str = "#include <hip/hip_runtime.h>";
        const SCALAR_HELPER: &str = r#"

__device__ __forceinline__ int mq8_scalar_mul(int a, int b) {
    int product;
    asm volatile("v_mul_lo_u32 %0, %1, %2" : "=v"(product) : "v"(a), "v"(b));
    return product;
}

__device__ __forceinline__ int mq8_dot4_scalar(int a, int b, int acc) {
    const unsigned int ua = (unsigned int)a;
    const unsigned int ub = (unsigned int)b;
    const int a0 = (int)(signed char)(ua & 0xffu);
    const int a1 = (int)(signed char)((ua >> 8) & 0xffu);
    const int a2 = (int)(signed char)((ua >> 16) & 0xffu);
    const int a3 = (int)(signed char)((ua >> 24) & 0xffu);
    const int b0 = (int)(signed char)(ub & 0xffu);
    const int b1 = (int)(signed char)((ub >> 8) & 0xffu);
    const int b2 = (int)(signed char)((ub >> 16) & 0xffu);
    const int b3 = (int)(signed char)((ub >> 24) & 0xffu);
    return acc + mq8_scalar_mul(a0, b0) + mq8_scalar_mul(a1, b1)
        + mq8_scalar_mul(a2, b2) + mq8_scalar_mul(a3, b3);
}
"#;
        source = source.replacen(INCLUDE, &format!("{INCLUDE}{SCALAR_HELPER}"), 1);
        let sudot = "return __builtin_amdgcn_sudot4(true, a, true, b, acc, false);";
        let sdot = "return __builtin_amdgcn_sdot4(a, b, acc, false);";
        assert!(source.contains(sudot) && source.contains(sdot));
        source = source.replace(sudot, "return mq8_dot4_scalar(a, b, acc);");
        source = source.replace(sdot, "return mq8_dot4_scalar(a, b, acc);");
    }
    source
}

fn gemv_args(
    weights: &GpuTensor,
    x_q8: &GpuTensor,
    x_scales: &GpuTensor,
    y: &GpuTensor,
    m: usize,
    k: usize,
) -> KernargBlob {
    let mut args = KernargBlob::new();
    args.push_ptr(weights.buf.as_ptr() as *const c_void);
    args.push_ptr(x_q8.buf.as_ptr() as *const c_void);
    args.push_ptr(x_scales.buf.as_ptr() as *const c_void);
    args.push_ptr(y.buf.as_ptr() as *const c_void);
    args.push_i32(m as i32);
    args.push_i32(k as i32);
    args.pad_to(16);
    args
}

fn launch(gpu: &Gpu, kernel: &str, args: &mut KernargBlob, m: usize) {
    gpu.launch_kernel_blob(kernel, [m as u32, 1, 1], [32, 1, 1], 0, args.as_mut_slice())
        .unwrap();
}

fn time_kernel(gpu: &Gpu, kernel: &str, args: &mut KernargBlob, m: usize, iters: usize) -> f64 {
    let start = gpu.hip.event_create().unwrap();
    let stop = gpu.hip.event_create().unwrap();
    gpu.hip.event_record(&start, None).unwrap();
    for _ in 0..iters {
        launch(gpu, kernel, args, m);
    }
    gpu.hip.event_record(&stop, None).unwrap();
    gpu.hip.event_synchronize(&stop).unwrap();
    let us = gpu.hip.event_elapsed_ms(&start, &stop).unwrap() as f64 * 1000.0 / iters as f64;
    gpu.hip.event_destroy(start).unwrap();
    gpu.hip.event_destroy(stop).unwrap();
    us
}

fn compare(a: &[f32], b: &[f32]) -> (f32, usize) {
    a.iter()
        .zip(b)
        .fold((0.0f32, 0usize), |(max_abs, exact), (&lhs, &rhs)| {
            (
                max_abs.max((lhs - rhs).abs()),
                exact + usize::from(lhs == rhs),
            )
        })
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}

fn synthetic_mq8(m: usize, k: usize, seed: u64) -> Vec<u8> {
    let groups_per_row = k / 256;
    let mut output = vec![0u8; m * groups_per_row * 258];
    let scale = f32_to_f16_bits(1.0 / 128.0).to_le_bytes();
    let mut state = seed;
    for row in 0..m {
        for group in 0..groups_per_row {
            let offset = (row * groups_per_row + group) * 258;
            output[offset..offset + 2].copy_from_slice(&scale);
            for value in &mut output[offset + 2..offset + 258] {
                *value = (((next_u32(&mut state) % 255) as i32 - 127) as i8) as u8;
            }
        }
    }
    output
}

fn synthetic_i8(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| (((next_u32(&mut state) % 255) as i32 - 127) as i8) as u8)
        .collect()
}

fn next_u32(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7fffff;
    if exponent <= 0 {
        return sign;
    }
    if exponent >= 31 {
        return sign | 0x7c00;
    }
    sign | (exponent as u16) << 10 | (mantissa >> 13) as u16
}
