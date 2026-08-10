// SPDX-License-Identifier: MIT
//! Synthetic GPU/CPU parity for classic HFQ weight formats.
//!
//! Unlike the historical Q4K/Q4F16 examples, this anchor has no external
//! GGUF dependency. It generates valid packed bytes in process and exercises
//! high scale/min bits, multiple rows and blocks, Q8 remainder groups, and
//! the 128-byte Q8HFQ row alignment contract.

use hipfire_runtime::llama::{
    convert_q4k_to_q4f16_g32, convert_q4k_to_q4f16_g64, dequantize_q4_k, f16_to_f32, f32_to_f16,
};
use rdna_compute::{DType, Gpu};

const Q4_TOLERANCE: f32 = 2.0e-3;
const Q8_TOLERANCE: f32 = 5.0e-4;

fn main() {
    if let Err(error) = run() {
        eprintln!("CLASSIC QUANT PARITY FAIL: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut gpu = Gpu::init().map_err(|error| format!("GPU init failed: {error}"))?;

    let q4_m = 9usize;
    let q4_k = 768usize;
    let q4k = build_q4k(q4_m, q4_k);
    let q4_x = deterministic_x(q4_k, 17);
    let q4_ref = matvec(&dequantize_q4_k(&q4k, q4_m * q4_k), &q4_x, q4_m, q4_k);
    let q4_gpu = gpu_q4k(&mut gpu, &q4k, &q4_x, q4_m, q4_k)?;
    let q4_error = require_close("Q4K", &q4_ref, &q4_gpu, Q4_TOLERANCE)?;

    let q4_g32 = convert_q4k_to_q4f16_g32(&q4k, q4_m * q4_k);
    let q4_g32_ref = matvec(&dequant_q4f16(&q4_g32, q4_m * q4_k, 32), &q4_x, q4_m, q4_k);
    let q4_g32_gpu = gpu_q4f16(&mut gpu, &q4_g32, &q4_x, q4_m, q4_k, 32)?;
    let q4_g32_error = require_close("Q4F16-G32", &q4_g32_ref, &q4_g32_gpu, Q4_TOLERANCE)?;

    let q4_g64 = convert_q4k_to_q4f16_g64(&q4k, q4_m * q4_k);
    let q4_g64_ref = matvec(&dequant_q4f16(&q4_g64, q4_m * q4_k, 64), &q4_x, q4_m, q4_k);
    let q4_g64_gpu = gpu_q4f16(&mut gpu, &q4_g64, &q4_x, q4_m, q4_k, 64)?;
    let q4_g64_error = require_close("Q4F16-G64", &q4_g64_ref, &q4_g64_gpu, Q4_TOLERANCE)?;

    let q8_m = 11usize;
    let q8_k = 416usize;
    let q8_x = deterministic_x(q8_k, 29);
    let (q8, q8_seen) = build_q8_0(q8_m, q8_k);
    let q8_ref = matvec(&q8_seen, &q8_x, q8_m, q8_k);
    let q8_gpu = gpu_q8(&mut gpu, &q8, &q8_x, q8_m, q8_k)?;
    let q8_error = require_close("Q8_0", &q8_ref, &q8_gpu, Q8_TOLERANCE)?;

    let (q8hfq, q8hfq_seen, row_stride) = build_q8hfq(q8_m, q8_k);
    let q8hfq_ref = matvec(&q8hfq_seen, &q8_x, q8_m, q8_k);
    let q8hfq_gpu = gpu_q8hfq(&mut gpu, &q8hfq, &q8_x, q8_m, q8_k, row_stride)?;
    let q8hfq_error = require_close("Q8HFQ", &q8hfq_ref, &q8hfq_gpu, Q8_TOLERANCE)?;

    eprintln!(
        "CLASSIC QUANT PARITY PASS: q4k={q4_error:.6e} q4g32={q4_g32_error:.6e} q4g64={q4_g64_error:.6e} q8={q8_error:.6e} q8hfq={q8hfq_error:.6e} q8hfq_stride={row_stride}"
    );
    Ok(())
}

fn build_q4k(m: usize, k: usize) -> Vec<u8> {
    assert_eq!(k % 256, 0);
    let scales = [3u8, 17, 31, 45, 7, 22, 38, 55];
    let mins = [1u8, 16, 29, 42, 6, 21, 37, 53];
    let blocks_per_row = k / 256;
    let mut packed = vec![0u8; m * blocks_per_row * 144];

    for row in 0..m {
        for block in 0..blocks_per_row {
            let offset = (row * blocks_per_row + block) * 144;
            let d = 0.0015 + row as f32 * 0.00007 + block as f32 * 0.00003;
            let dmin = 0.0009 + row as f32 * 0.00002;
            packed[offset..offset + 2].copy_from_slice(&f32_to_f16(d).to_le_bytes());
            packed[offset + 2..offset + 4].copy_from_slice(&f32_to_f16(dmin).to_le_bytes());

            for index in 0..4 {
                packed[offset + 4 + index] = (scales[index] & 63) | ((scales[4 + index] >> 4) << 6);
                packed[offset + 8 + index] = (mins[index] & 63) | ((mins[4 + index] >> 4) << 6);
                packed[offset + 12 + index] =
                    (scales[4 + index] & 15) | ((mins[4 + index] & 15) << 4);
            }
            for group in 0..4 {
                for lane in 0..32 {
                    let lo = ((row * 11 + block * 7 + group * 5 + lane * 3) & 15) as u8;
                    let hi = ((row * 13 + block * 9 + group * 7 + lane * 5 + 1) & 15) as u8;
                    packed[offset + 16 + group * 32 + lane] = lo | (hi << 4);
                }
            }
        }
    }
    packed
}

fn dequant_q4f16(data: &[u8], elements: usize, group_size: usize) -> Vec<f32> {
    let block_bytes = 4 + group_size / 2;
    let mut values = vec![0.0f32; elements];
    for block in 0..elements / group_size {
        let offset = block * block_bytes;
        let scale = f16_to_f32(u16::from_le_bytes([data[offset], data[offset + 1]]));
        let minimum = f16_to_f32(u16::from_le_bytes([data[offset + 2], data[offset + 3]]));
        for index in 0..group_size {
            let packed_index = index % (group_size / 2);
            let byte = data[offset + 4 + packed_index];
            let quant = if index < group_size / 2 {
                byte & 15
            } else {
                byte >> 4
            };
            let weight = quant as f32 * scale + minimum;
            values[block * group_size + index] = f16_to_f32(f32_to_f16(weight));
        }
    }
    values
}

fn build_q8_0(m: usize, k: usize) -> (Vec<u8>, Vec<f32>) {
    assert_eq!(k % 32, 0);
    let groups_per_row = k / 32;
    let mut packed = vec![0u8; m * groups_per_row * 34];
    let mut seen = vec![0.0f32; m * k];
    for row in 0..m {
        for group in 0..groups_per_row {
            let offset = (row * groups_per_row + group) * 34;
            let scale_bits = f32_to_f16(0.0025 + row as f32 * 0.0001 + group as f32 * 0.00001);
            let scale = f16_to_f32(scale_bits);
            packed[offset..offset + 2].copy_from_slice(&scale_bits.to_le_bytes());
            for lane in 0..32 {
                let quant = (((row * 23 + group * 17 + lane * 11) % 255) as i16 - 127) as i8;
                packed[offset + 2 + lane] = quant as u8;
                seen[row * k + group * 32 + lane] = scale * quant as f32;
            }
        }
    }
    (packed, seen)
}

fn build_q8hfq(m: usize, k: usize) -> (Vec<u8>, Vec<f32>, usize) {
    assert_eq!(k % 32, 0);
    let groups_per_row = k / 32;
    let scales_bytes = groups_per_row * 2;
    let row_stride = (scales_bytes + k + 127) & !127;
    let mut packed = vec![0u8; m * row_stride];
    let mut seen = vec![0.0f32; m * k];
    for row in 0..m {
        let row_offset = row * row_stride;
        for group in 0..groups_per_row {
            let scale_bits = f32_to_f16(0.0017 + row as f32 * 0.00008 + group as f32 * 0.00002);
            let scale = f16_to_f32(scale_bits);
            let scale_offset = row_offset + group * 2;
            packed[scale_offset..scale_offset + 2].copy_from_slice(&scale_bits.to_le_bytes());
            for lane in 0..32 {
                let quant = (((row * 19 + group * 29 + lane * 7) % 255) as i16 - 127) as i8;
                packed[row_offset + scales_bytes + group * 32 + lane] = quant as u8;
                seen[row * k + group * 32 + lane] = scale * quant as f32;
            }
        }
    }
    (packed, seen, row_stride)
}

fn deterministic_x(k: usize, stride: usize) -> Vec<f32> {
    (0..k)
        .map(|index| (((index * stride + 5) % 101) as f32 - 50.0) * 0.002)
        .collect()
}

fn matvec(weights: &[f32], x: &[f32], m: usize, k: usize) -> Vec<f32> {
    (0..m)
        .map(|row| {
            weights[row * k..(row + 1) * k]
                .iter()
                .zip(x)
                .map(|(&weight, &activation)| weight * activation)
                .sum()
        })
        .collect()
}

fn require_close(
    label: &str,
    expected: &[f32],
    actual: &[f32],
    tolerance: f32,
) -> Result<f32, String> {
    if actual.iter().any(|value| !value.is_finite()) {
        return Err(format!("{label} produced a non-finite value"));
    }
    let max_error = expected
        .iter()
        .zip(actual)
        .map(|(&left, &right)| (left - right).abs())
        .fold(0.0f32, f32::max);
    if max_error > tolerance {
        return Err(format!(
            "{label} max_abs={max_error:.6e} exceeds {tolerance:.6e}"
        ));
    }
    Ok(max_error)
}

fn download(gpu: &Gpu, tensor: &rdna_compute::GpuTensor) -> Result<Vec<f32>, String> {
    gpu.download_f32(tensor)
        .map_err(|error| format!("download failed: {error}"))
}

fn gpu_q4k(
    gpu: &mut Gpu,
    weights: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
) -> Result<Vec<f32>, String> {
    let a = gpu
        .upload_raw(weights, &[weights.len()])
        .map_err(|error| error.to_string())?;
    let x = gpu.upload_f32(x, &[k]).map_err(|error| error.to_string())?;
    let y = gpu
        .zeros(&[m], DType::F32)
        .map_err(|error| error.to_string())?;
    gpu.gemv_q4k(&a, &x, &y, m, k)
        .map_err(|error| error.to_string())?;
    download(gpu, &y)
}

fn gpu_q4f16(
    gpu: &mut Gpu,
    weights: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
    group_size: usize,
) -> Result<Vec<f32>, String> {
    let a = gpu
        .upload_raw(weights, &[weights.len()])
        .map_err(|error| error.to_string())?;
    let x = gpu.upload_f32(x, &[k]).map_err(|error| error.to_string())?;
    let y = gpu
        .zeros(&[m], DType::F32)
        .map_err(|error| error.to_string())?;
    match group_size {
        32 => gpu.gemv_q4f16_g32(&a, &x, &y, m, k),
        64 => gpu.gemv_q4f16_g64(&a, &x, &y, m, k),
        _ => unreachable!(),
    }
    .map_err(|error| error.to_string())?;
    download(gpu, &y)
}

fn gpu_q8(
    gpu: &mut Gpu,
    weights: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
) -> Result<Vec<f32>, String> {
    let a = gpu
        .upload_raw(weights, &[weights.len()])
        .map_err(|error| error.to_string())?;
    let x = gpu.upload_f32(x, &[k]).map_err(|error| error.to_string())?;
    let y = gpu
        .zeros(&[m], DType::F32)
        .map_err(|error| error.to_string())?;
    gpu.gemv_q8_0(&a, &x, &y, m, k)
        .map_err(|error| error.to_string())?;
    download(gpu, &y)
}

fn gpu_q8hfq(
    gpu: &mut Gpu,
    weights: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
    row_stride: usize,
) -> Result<Vec<f32>, String> {
    let a = gpu
        .upload_raw(weights, &[weights.len()])
        .map_err(|error| error.to_string())?;
    let x = gpu.upload_f32(x, &[k]).map_err(|error| error.to_string())?;
    let y = gpu
        .zeros(&[m], DType::F32)
        .map_err(|error| error.to_string())?;
    gpu.gemv_q8hfq(&a, &x, &y, m, k, row_stride)
        .map_err(|error| error.to_string())?;
    download(gpu, &y)
}
