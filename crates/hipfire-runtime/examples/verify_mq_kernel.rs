// SPDX-License-Identifier: MIT
//! Q0 — Numerical kernel verification for MQ8/MQ6/MQ4/MQ3/MQ2 GEMV pipelines.
//!
//! Generates synthetic weights, quantizes to the registered MQ integer formats, then compares
//! the GPU rotated GEMV output against a CPU reference
//! that reconstructs the rotated weights and rotates x using the same
//! FWHT math as the quantizer.
//!
//! Run:
//!   cargo run --release --example verify_mq_kernel
//!
//! Acceptance (per Q0):
//!   max_abs_err <= 1e-3 for MQ8/MQ6/MQ3/MQ2 and rotation.
//!   max_abs_err <= 2e-3 for MQ4, whose FP32 lane-tree reduction differs from
//!   the independent serial CPU accumulation over as many as 1024 terms.

fn main() {
    let mut gpu = rdna_compute::Gpu::init().unwrap();

    // Test multiple shapes: 1-group, 2-group, 4-group rows.
    let shapes = [(4usize, 256usize), (4, 512), (8, 1024)];

    let mut any_fail = false;

    for &(m, k) in &shapes {
        eprintln!("\n========== shape {} x {} ==========", m, k);

        // Deterministic pseudo-random weights and input (reproducible across runs)
        let f32_weights: Vec<f32> = (0..m * k)
            .map(|i| fract_sin(i as f32 * 0.731f32 + 1.337f32))
            .collect();
        let x: Vec<f32> = (0..k)
            .map(|i| fract_sin(i as f32 * 0.513f32 + 2.719f32))
            .collect();

        // ---- MQ8 ----
        let mq8_bytes = quantize_mq8g256(&f32_weights, k);
        let y_mq8_cpu = cpu_reference_mq8(&mq8_bytes, &x, m, k);
        let y_mq8_gpu = gpu_mq_gemv(&mut gpu, &mq8_bytes, &x, m, k, rdna_compute::DType::MQ8G256);
        let (ok8, _, _) = compare("MQ8", &y_mq8_cpu, &y_mq8_gpu, 1e-3);
        any_fail |= !ok8;

        // ---- MQ6 ----
        let mq6_bytes = quantize_mq6g256(&f32_weights, k);
        let y_mq6_cpu = cpu_reference_mq(&mq6_bytes, &x, m, k, 200, 6, |scale, zero, q| {
            scale * q as f32 + zero
        });
        let y_mq6_gpu = gpu_mq_gemv(&mut gpu, &mq6_bytes, &x, m, k, rdna_compute::DType::MQ6G256);
        let (ok6, _, _) = compare("MQ6", &y_mq6_cpu, &y_mq6_gpu, 1e-3);
        any_fail |= !ok6;

        // ---- MQ4 ----
        let mq4_bytes = quantize_mq4g256(&f32_weights, k);
        let y_mq4_cpu = cpu_reference_mq(&mq4_bytes, &x, m, k, 136, 4, |scale, zero, q| {
            scale * q as f32 + zero
        });
        let y_mq4_gpu = gpu_mq_gemv(&mut gpu, &mq4_bytes, &x, m, k, rdna_compute::DType::MQ4G256);
        let (ok4, _, _) = compare("MQ4", &y_mq4_cpu, &y_mq4_gpu, 2e-3);
        any_fail |= !ok4;

        // ---- MQ3 ----
        let mq3_bytes = quantize_mq3g256(&f32_weights, k);
        let y_mq3_cpu = cpu_reference_mq(&mq3_bytes, &x, m, k, 104, 3, |scale, zero, q| {
            scale * q as f32 + zero
        });
        let y_mq3_gpu = gpu_mq_gemv(&mut gpu, &mq3_bytes, &x, m, k, rdna_compute::DType::MQ3G256);
        let (ok3, _, _) = compare("MQ3", &y_mq3_cpu, &y_mq3_gpu, 1e-3);
        any_fail |= !ok3;

        // ---- MQ2 ----
        let mq2_bytes = quantize_mq2g256(&f32_weights, k);
        let y_mq2_cpu = cpu_reference_mq(&mq2_bytes, &x, m, k, 72, 2, |scale, zero, q| {
            scale * q as f32 + zero
        });
        let y_mq2_gpu = gpu_mq_gemv(&mut gpu, &mq2_bytes, &x, m, k, rdna_compute::DType::MQ2G256);
        let (ok2, _, _) = compare("MQ2", &y_mq2_cpu, &y_mq2_gpu, 1e-3);
        any_fail |= !ok2;

        // Also verify the *rotation-only* step in isolation.
        let x_rot_cpu = cpu_rotate_x_mq(&x);
        let x_rot_gpu = gpu_rotate_x_mq(&mut gpu, &x, k);
        let (ok_rot, max_err_rot, _) = compare("rot", &x_rot_cpu, &x_rot_gpu, 1e-3);
        any_fail |= !ok_rot;
        eprintln!("  rotate_x max_err={:.6e}", max_err_rot);
    }

    if any_fail {
        eprintln!("\n[FAIL] One or more checks exceeded its format-specific threshold.");
        std::process::exit(1);
    } else {
        eprintln!("\n[PASS] All MQ8/MQ6/MQ4/MQ3/MQ2 kernels meet their parity budgets.");
    }
}

fn fract_sin(x: f32) -> f32 {
    (x.sin() * 12345.6789f32).fract() * 2.0f32 - 1.0f32
}

fn compare(name: &str, cpu: &[f32], gpu: &[f32], tolerance: f32) -> (bool, f32, f32) {
    let mut max_err = 0.0f32;
    let mut sum_err = 0.0f32;
    let mut bit_exact = 0usize;
    for i in 0..cpu.len() {
        let err = (cpu[i] - gpu[i]).abs();
        max_err = max_err.max(err);
        sum_err += err;
        if cpu[i] == gpu[i] {
            bit_exact += 1;
        }
    }
    let mean_err = sum_err / cpu.len().max(1) as f32;
    let ok = max_err <= tolerance;
    let status = if ok { "PASS" } else { "FAIL" };
    eprintln!(
        "  {:<6} {}  max_err={:.6e}  mean_err={:.6e}  tolerance={:.1e}  bit_exact={}/{}",
        name,
        status,
        max_err,
        mean_err,
        tolerance,
        bit_exact,
        cpu.len()
    );
    (ok, max_err, mean_err)
}

// ---------------------------------------------------------------------------
// CPU reference: rotate x, dequantize weights, compute y = W_rot * x_rot
// ---------------------------------------------------------------------------

fn cpu_reference_mq(
    bytes: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
    group_bytes: usize,
    bits: u8,
    recon: impl Fn(f32, f32, u8) -> f32,
) -> Vec<f32> {
    let groups_per_row = k / 256;
    let mut y = vec![0.0f32; m];

    // Rotate x
    let x_rot = cpu_rotate_x_mq(x);

    for row in 0..m {
        let row_off = row * groups_per_row * group_bytes;
        let mut acc = 0.0f32;

        for g in 0..groups_per_row {
            let g_off = row_off + g * group_bytes;
            let scale = f32::from_le_bytes([
                bytes[g_off],
                bytes[g_off + 1],
                bytes[g_off + 2],
                bytes[g_off + 3],
            ]);
            let zero = f32::from_le_bytes([
                bytes[g_off + 4],
                bytes[g_off + 5],
                bytes[g_off + 6],
                bytes[g_off + 7],
            ]);
            let data = &bytes[g_off + 8..g_off + group_bytes];

            let base_idx = g * 256;
            let mut q_vals: Vec<u8> = Vec::with_capacity(256);

            if bits == 6 {
                // 256 weights = 64 chunks * 4 weights * 6 bits = 192 bytes
                for chunk in 0..64 {
                    let b0 = data[chunk * 3] as u32;
                    let b1 = data[chunk * 3 + 1] as u32;
                    let b2 = data[chunk * 3 + 2] as u32;
                    q_vals.push((b0 & 0x3f) as u8);
                    q_vals.push((((b0 >> 6) | (b1 << 2)) & 0x3f) as u8);
                    q_vals.push((((b1 >> 4) | (b2 << 4)) & 0x3f) as u8);
                    q_vals.push(((b2 >> 2) & 0x3f) as u8);
                }
            } else if bits == 4 {
                for &byte in data.iter().take(128) {
                    q_vals.push(byte & 0x0f);
                    q_vals.push(byte >> 4);
                }
            } else if bits == 3 {
                // 256 weights = 32 chunks * 8 weights * 3 bits = 96 bytes
                for chunk in 0..32 {
                    let b0 = data[chunk * 3];
                    let b1 = data[chunk * 3 + 1];
                    let b2 = data[chunk * 3 + 2];
                    q_vals.push(b0 & 7);
                    q_vals.push((b0 >> 3) & 7);
                    q_vals.push(((b0 >> 6) | (b1 << 2)) & 7);
                    q_vals.push((b1 >> 1) & 7);
                    q_vals.push((b1 >> 4) & 7);
                    q_vals.push(((b1 >> 7) | (b2 << 1)) & 7);
                    q_vals.push((b2 >> 2) & 7);
                    q_vals.push((b2 >> 5) & 7);
                }
            } else if bits == 2 {
                // 256 weights = 64 bytes, 4 weights per byte
                for i in 0..64 {
                    let b = data[i];
                    q_vals.push(b & 3);
                    q_vals.push((b >> 2) & 3);
                    q_vals.push((b >> 4) & 3);
                    q_vals.push((b >> 6) & 3);
                }
            } else {
                panic!("unsupported bits {}", bits);
            }

            for j in 0..256 {
                let w = recon(scale, zero, q_vals[j]);
                acc += w * x_rot[base_idx + j];
            }
        }
        y[row] = acc;
    }
    y
}

fn cpu_reference_mq8(bytes: &[u8], x: &[f32], m: usize, k: usize) -> Vec<f32> {
    let groups_per_row = k / 256;
    let x_rot = cpu_rotate_x_mq(x);
    let mut x_q = vec![0i8; k];
    let mut x_scales = vec![0.0f32; groups_per_row];
    for group in 0..groups_per_row {
        let values = &x_rot[group * 256..(group + 1) * 256];
        let amax = values
            .iter()
            .fold(0.0f32, |acc, value| acc.max(value.abs()));
        let scale = if amax > 0.0 { amax / 127.0 } else { 1.0 };
        let inverse = if amax > 0.0 { 127.0 / amax } else { 0.0 };
        x_scales[group] = scale;
        for (index, value) in values.iter().enumerate() {
            x_q[group * 256 + index] = (value * inverse).round().clamp(-128.0, 127.0) as i8;
        }
    }

    let mut y = vec![0.0f32; m];
    for row in 0..m {
        for group in 0..groups_per_row {
            let offset = (row * groups_per_row + group) * 258;
            let scale_bits = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            let weight_scale = hipfire_runtime::llama::f16_to_f32(scale_bits);
            let mut dot = 0i32;
            for index in 0..256 {
                dot += (bytes[offset + 2 + index] as i8 as i32) * (x_q[group * 256 + index] as i32);
            }
            y[row] += weight_scale * x_scales[group] * dot as f32;
        }
    }
    y
}

fn cpu_rotate_x_mq(x: &[f32]) -> Vec<f32> {
    let k = x.len();
    assert!(k % 256 == 0, "k must be multiple of 256");
    let signs1 = gen_fwht_signs(42, 256);
    let signs2 = gen_fwht_signs(1042, 256);
    let mut out = vec![0.0f32; k];
    for g in 0..(k / 256) {
        let mut group = [0.0f32; 256];
        group.copy_from_slice(&x[g * 256..(g + 1) * 256]);
        cpu_fwht_256(&mut group, &signs1, &signs2);
        out[g * 256..(g + 1) * 256].copy_from_slice(&group);
    }
    out
}

fn gen_fwht_signs(seed: u32, n: usize) -> Vec<f32> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
            if (state >> 16) & 1 == 1 {
                1.0f32
            } else {
                -1.0f32
            }
        })
        .collect()
}

fn cpu_fwht_256(x: &mut [f32], signs1: &[f32], signs2: &[f32]) {
    assert!(x.len() == 256);
    for i in 0..256 {
        x[i] *= signs1[i];
    }
    let mut stride = 1;
    while stride < 256 {
        let mut i = 0;
        while i < 256 {
            for j in 0..stride {
                let a = x[i + j];
                let b = x[i + j + stride];
                x[i + j] = a + b;
                x[i + j + stride] = a - b;
            }
            i += stride * 2;
        }
        stride <<= 1;
    }
    let scale = 0.0625; // 1/16
    for i in 0..256 {
        x[i] *= scale * signs2[i];
    }
}

// ---------------------------------------------------------------------------
// GPU wrappers
// ---------------------------------------------------------------------------

fn gpu_mq_gemv(
    gpu: &mut rdna_compute::Gpu,
    bytes: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
    dtype: rdna_compute::DType,
) -> Vec<f32> {
    let d_a = gpu.upload_raw(bytes, &[bytes.len()]).unwrap();
    let d_x = gpu.upload_f32(x, &[k]).unwrap();
    let d_y = gpu.zeros(&[m], rdna_compute::DType::F32).unwrap();

    match dtype {
        rdna_compute::DType::MQ8G256 => {
            gpu.gemv_mq8g256_with_rotate(&d_a, &d_x, &d_y, m, k)
                .unwrap();
        }
        rdna_compute::DType::MQ6G256 => {
            let d_tmp = gpu.zeros(&[k], rdna_compute::DType::F32).unwrap();
            gpu.gemv_mq6g256_with_rotate(&d_a, &d_x, &d_y, &d_tmp, m, k)
                .unwrap();
        }
        rdna_compute::DType::MQ4G256 => {
            let d_tmp = gpu.zeros(&[k], rdna_compute::DType::F32).unwrap();
            gpu.gemv_mq4g256_with_rotate(&d_a, &d_x, &d_y, &d_tmp, m, k)
                .unwrap();
        }
        rdna_compute::DType::MQ3G256 => {
            let d_tmp = gpu.zeros(&[k], rdna_compute::DType::F32).unwrap();
            gpu.gemv_mq3g256_with_rotate(&d_a, &d_x, &d_y, &d_tmp, m, k)
                .unwrap();
        }
        rdna_compute::DType::MQ2G256 => {
            let d_tmp = gpu.zeros(&[k], rdna_compute::DType::F32).unwrap();
            gpu.gemv_mq2g256_with_rotate(&d_a, &d_x, &d_y, &d_tmp, m, k)
                .unwrap();
        }
        _ => panic!("unexpected dtype"),
    }

    let mut y = vec![0.0f32; m];
    let y_bytes = unsafe { std::slice::from_raw_parts_mut(y.as_mut_ptr() as *mut u8, m * 4) };
    gpu.hip.memcpy_dtoh(y_bytes, &d_y.buf).unwrap();
    y
}

fn gpu_rotate_x_mq(gpu: &mut rdna_compute::Gpu, x: &[f32], k: usize) -> Vec<f32> {
    let d_x = gpu.upload_f32(x, &[k]).unwrap();
    let d_xr = gpu.zeros(&[k], rdna_compute::DType::F32).unwrap();
    gpu.rotate_x_mq(&d_x, &d_xr, k).unwrap();
    let mut out = vec![0.0f32; k];
    let out_bytes = unsafe { std::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, k * 4) };
    gpu.hip.memcpy_dtoh(out_bytes, &d_xr.buf).unwrap();
    out
}

// ---------------------------------------------------------------------------
// Quantizers (mirroring hipfire-quantize/src/main.rs)
// ---------------------------------------------------------------------------

fn quantize_mq8g256(f32_data: &[f32], _k: usize) -> Vec<u8> {
    let mut output = vec![0u8; f32_data.len().div_ceil(256) * 258];
    let signs1 = gen_fwht_signs(42, 256);
    let signs2 = gen_fwht_signs(1042, 256);
    for (block, values) in f32_data.chunks(256).enumerate() {
        let mut group = [0.0f32; 256];
        group[..values.len()].copy_from_slice(values);
        cpu_fwht_256(&mut group, &signs1, &signs2);
        let amax = group.iter().fold(0.0f32, |acc, value| acc.max(value.abs()));
        let scale = if amax > 0.0 { amax / 127.0 } else { 1.0 };
        let inverse = if amax > 0.0 { 127.0 / amax } else { 0.0 };
        let offset = block * 258;
        output[offset..offset + 2].copy_from_slice(&f32_to_f16(scale).to_le_bytes());
        for (index, value) in group.into_iter().enumerate() {
            output[offset + 2 + index] = (value * inverse).round().clamp(-128.0, 127.0) as i8 as u8;
        }
    }
    output
}

fn quantize_mq6g256(f32_data: &[f32], _k: usize) -> Vec<u8> {
    let group_size = 256;
    let block_bytes = 200;
    let n_blocks = f32_data.len().div_ceil(group_size);
    let mut output = vec![0u8; n_blocks * block_bytes];
    let signs1 = gen_fwht_signs(42, 256);
    let signs2 = gen_fwht_signs(1042, 256);

    for b in 0..n_blocks {
        let start = b * group_size;
        let end = (start + group_size).min(f32_data.len());
        let mut group = [0.0f32; 256];
        group[..end - start].copy_from_slice(&f32_data[start..end]);
        cpu_fwht_256(&mut group, &signs1, &signs2);

        let min_val = group.iter().copied().fold(f32::INFINITY, f32::min);
        let max_val = group.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = max_val - min_val;
        let scale = if range > 0.0 { range / 63.0 } else { 1.0 };
        let inv_scale = if range > 0.0 { 1.0 / scale } else { 0.0 };
        let out_off = b * block_bytes;
        output[out_off..out_off + 4].copy_from_slice(&scale.to_le_bytes());
        output[out_off + 4..out_off + 8].copy_from_slice(&min_val.to_le_bytes());

        for chunk in 0..64 {
            let i = chunk * 4;
            let q0 = ((group[i] - min_val) * inv_scale + 0.5).clamp(0.0, 63.0) as u8;
            let q1 = ((group[i + 1] - min_val) * inv_scale + 0.5).clamp(0.0, 63.0) as u8;
            let q2 = ((group[i + 2] - min_val) * inv_scale + 0.5).clamp(0.0, 63.0) as u8;
            let q3 = ((group[i + 3] - min_val) * inv_scale + 0.5).clamp(0.0, 63.0) as u8;
            let byte_off = out_off + 8 + chunk * 3;
            output[byte_off] = q0 | (q1 << 6);
            output[byte_off + 1] = (q1 >> 2) | (q2 << 4);
            output[byte_off + 2] = (q2 >> 4) | (q3 << 2);
        }
    }
    output
}

fn quantize_mq4g256(f32_data: &[f32], _k: usize) -> Vec<u8> {
    let mut output = vec![0u8; f32_data.len().div_ceil(256) * 136];
    let signs1 = gen_fwht_signs(42, 256);
    let signs2 = gen_fwht_signs(1042, 256);
    for (block, values) in f32_data.chunks(256).enumerate() {
        let mut group = [0.0f32; 256];
        group[..values.len()].copy_from_slice(values);
        cpu_fwht_256(&mut group, &signs1, &signs2);
        let min = group.iter().copied().fold(f32::INFINITY, f32::min);
        let max = group.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;
        let scale = if range > 0.0 { range / 15.0 } else { 1.0 };
        let inverse = if range > 0.0 { scale.recip() } else { 0.0 };
        let offset = block * 136;
        output[offset..offset + 4].copy_from_slice(&scale.to_le_bytes());
        output[offset + 4..offset + 8].copy_from_slice(&min.to_le_bytes());
        for index in 0..128 {
            let low = ((group[index * 2] - min) * inverse + 0.5).clamp(0.0, 15.0) as u8;
            let high = ((group[index * 2 + 1] - min) * inverse + 0.5).clamp(0.0, 15.0) as u8;
            output[offset + 8 + index] = low | high << 4;
        }
    }
    output
}

fn f32_to_f16(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let mut exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7fffff;
    if exponent >= 31 {
        return sign | 0x7c00;
    }
    if exponent <= 0 {
        if exponent < -10 {
            return sign;
        }
        let full = mantissa | 0x800000;
        let shift = (1 - exponent) as u32 + 13;
        let mut rounded = (full >> shift) as u16;
        let lost = full & ((1u32 << shift) - 1);
        let half = 1u32 << (shift - 1);
        if lost > half || (lost == half && rounded & 1 != 0) {
            rounded = rounded.wrapping_add(1);
        }
        return sign | rounded;
    }
    let mut rounded = (mantissa >> 13) as u16;
    let lost = mantissa & 0x1fff;
    if lost > 0x1000 || (lost == 0x1000 && rounded & 1 != 0) {
        rounded += 1;
        if rounded == 0x400 {
            rounded = 0;
            exponent += 1;
        }
    }
    sign | (exponent as u16) << 10 | rounded
}

fn quantize_mq3g256(f32_data: &[f32], _k: usize) -> Vec<u8> {
    let group_size = 256;
    let block_bytes = 104;
    let n = f32_data.len();
    let n_blocks = (n + group_size - 1) / group_size;
    let mut output = vec![0u8; n_blocks * block_bytes];
    let signs1 = gen_fwht_signs(42, 256);
    let signs2 = gen_fwht_signs(1042, 256);

    for b in 0..n_blocks {
        let start = b * group_size;
        let end = (start + group_size).min(n);
        let mut group = [0.0f32; 256];
        let actual_len = end - start;
        group[..actual_len].copy_from_slice(&f32_data[start..end]);
        cpu_fwht_256(&mut group, &signs1, &signs2);

        let min_val = group.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_val = group.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max_val - min_val;
        let scale = if range > 0.0 { range / 7.0 } else { 1.0 };
        let inv_scale = if range > 0.0 { 1.0 / scale } else { 0.0 };

        let out_off = b * block_bytes;
        output[out_off..out_off + 4].copy_from_slice(&scale.to_le_bytes());
        output[out_off + 4..out_off + 8].copy_from_slice(&min_val.to_le_bytes());

        for chunk in 0..32 {
            let ci = chunk * 8;
            let mut q = [0u8; 8];
            for j in 0..8 {
                q[j] = ((group[ci + j] - min_val) * inv_scale + 0.5).clamp(0.0, 7.0) as u8;
            }
            let b0 = (q[0] & 7) | ((q[1] & 7) << 3) | ((q[2] & 3) << 6);
            let b1 = ((q[2] >> 2) & 1) | ((q[3] & 7) << 1) | ((q[4] & 7) << 4) | ((q[5] & 1) << 7);
            let b2 = ((q[5] >> 1) & 3) | ((q[6] & 7) << 2) | ((q[7] & 7) << 5);

            let bo = out_off + 8 + chunk * 3;
            output[bo] = b0;
            output[bo + 1] = b1;
            output[bo + 2] = b2;
        }
    }
    output
}

fn quantize_mq2g256(f32_data: &[f32], _k: usize) -> Vec<u8> {
    let group_size = 256;
    let block_bytes = 72;
    let n = f32_data.len();
    let n_blocks = (n + group_size - 1) / group_size;
    let mut output = vec![0u8; n_blocks * block_bytes];
    let signs1 = gen_fwht_signs(42, 256);
    let signs2 = gen_fwht_signs(1042, 256);

    for b in 0..n_blocks {
        let start = b * group_size;
        let end = (start + group_size).min(n);
        let mut group = [0.0f32; 256];
        let actual_len = end - start;
        group[..actual_len].copy_from_slice(&f32_data[start..end]);
        cpu_fwht_256(&mut group, &signs1, &signs2);

        let min_val = group.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_val = group.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max_val - min_val;
        let scale = if range > 0.0 { range / 3.0 } else { 1.0 };
        let inv_scale = if range > 0.0 { 1.0 / scale } else { 0.0 };

        let out_off = b * block_bytes;
        output[out_off..out_off + 4].copy_from_slice(&scale.to_le_bytes());
        output[out_off + 4..out_off + 8].copy_from_slice(&min_val.to_le_bytes());

        for i in 0..64 {
            let mut byte_val = 0u8;
            for j in 0..4 {
                let q = ((group[4 * i + j] - min_val) * inv_scale + 0.5) as u8;
                byte_val |= q.min(3) << (j * 2);
            }
            output[out_off + 8 + i] = byte_val;
        }
    }
    output
}
