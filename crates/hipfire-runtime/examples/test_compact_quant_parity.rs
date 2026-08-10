// SPDX-License-Identifier: MIT
//! CPU-reference/GPU parity for compact HFQ formats and MQ2-Lloyd.

use rdna_compute::{DType, Gpu};

#[derive(Clone, Copy)]
struct UniformFormat {
    name: &'static str,
    dtype: DType,
    bits: usize,
    group: usize,
}

fn main() {
    let mut gpu = Gpu::init().expect("GPU init");
    let formats = [
        UniformFormat {
            name: "HFQ4-G128",
            dtype: DType::HFQ4G128,
            bits: 4,
            group: 128,
        },
        UniformFormat {
            name: "HFQ2-G256",
            dtype: DType::HFQ2G256,
            bits: 2,
            group: 256,
        },
        UniformFormat {
            name: "HFQ2-G128",
            dtype: DType::HFQ2G128,
            bits: 2,
            group: 128,
        },
        UniformFormat {
            name: "HFQ3-G128",
            dtype: DType::HFQ3G128,
            bits: 3,
            group: 128,
        },
    ];
    let mut failed = false;

    for format in formats {
        for k in [256usize, 512, 1024] {
            failed |= !run_uniform(&mut gpu, format, 5, k);
        }
    }
    for k in [256usize, 512, 1024] {
        failed |= !run_mq2_lloyd(&mut gpu, 5, k);
    }

    if failed {
        eprintln!("[FAIL] compact quantization parity exceeded 2e-3");
        std::process::exit(1);
    }
    eprintln!("[PASS] compact HFQ and MQ2-Lloyd parity");
}

fn signal(i: usize, salt: f32) -> f32 {
    ((i as f32 * 0.617 + salt).sin() * 8191.371).fract() * 2.0 - 1.0
}

fn run_uniform(gpu: &mut Gpu, format: UniformFormat, m: usize, k: usize) -> bool {
    let weights: Vec<f32> = (0..m * k).map(|i| signal(i, 0.73)).collect();
    let x: Vec<f32> = (0..k).map(|i| signal(i, 2.11)).collect();
    let bytes = quantize_uniform(&weights, format.bits, format.group);
    let cpu = reference_uniform(&bytes, &x, m, k, format.bits, format.group);
    let gpu_out = gpu_uniform(gpu, &bytes, &x, m, k, format.dtype);
    compare(format.name, k, &cpu, &gpu_out)
}

fn quantize_uniform(values: &[f32], bits: usize, group: usize) -> Vec<u8> {
    let payload_bytes = group * bits / 8;
    let block_bytes = 8 + payload_bytes;
    let mut output = vec![0u8; values.len().div_ceil(group) * block_bytes];
    let levels = (1u32 << bits) - 1;

    for (block, chunk) in values.chunks(group).enumerate() {
        let min = chunk.iter().copied().fold(f32::INFINITY, f32::min);
        let max = chunk.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;
        let scale = if range > 0.0 {
            range / levels as f32
        } else {
            1.0
        };
        let inverse = if range > 0.0 { scale.recip() } else { 0.0 };
        let offset = block * block_bytes;
        output[offset..offset + 4].copy_from_slice(&scale.to_le_bytes());
        output[offset + 4..offset + 8].copy_from_slice(&min.to_le_bytes());
        for i in 0..group {
            let value = chunk.get(i).copied().unwrap_or(min);
            let q = ((value - min) * inverse + 0.5).clamp(0.0, levels as f32) as u8;
            put_bits(
                &mut output[offset + 8..offset + block_bytes],
                i * bits,
                bits,
                q,
            );
        }
    }
    output
}

fn reference_uniform(
    bytes: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
    bits: usize,
    group: usize,
) -> Vec<f32> {
    let block_bytes = 8 + group * bits / 8;
    let groups_per_row = k / group;
    let mut y = vec![0.0f32; m];
    for row in 0..m {
        for g in 0..groups_per_row {
            let offset = (row * groups_per_row + g) * block_bytes;
            let scale = read_f32(bytes, offset);
            let zero = read_f32(bytes, offset + 4);
            let payload = &bytes[offset + 8..offset + block_bytes];
            for j in 0..group {
                let q = get_bits(payload, j * bits, bits);
                y[row] += (scale * q as f32 + zero) * x[g * group + j];
            }
        }
    }
    y
}

fn gpu_uniform(
    gpu: &mut Gpu,
    bytes: &[u8],
    x: &[f32],
    m: usize,
    k: usize,
    dtype: DType,
) -> Vec<f32> {
    let a = gpu.upload_raw(bytes, &[bytes.len()]).unwrap();
    let x = gpu.upload_f32(x, &[k]).unwrap();
    let y = gpu.zeros(&[m], DType::F32).unwrap();
    match dtype {
        DType::HFQ4G128 => gpu.gemv_hfq4g128(&a, &x, &y, m, k).unwrap(),
        DType::HFQ2G256 => gpu.gemv_hfq2g256(&a, &x, &y, m, k).unwrap(),
        DType::HFQ2G128 => gpu.gemv_hfq2g128(&a, &x, &y, m, k).unwrap(),
        DType::HFQ3G128 => gpu.gemv_hfq3g128(&a, &x, &y, m, k).unwrap(),
        _ => unreachable!(),
    }
    download(gpu, &y, m)
}

fn run_mq2_lloyd(gpu: &mut Gpu, m: usize, k: usize) -> bool {
    let x: Vec<f32> = (0..k).map(|i| signal(i, 4.37)).collect();
    let x_rot = rotate_x(&x);
    let groups_per_row = k / 256;
    let mut bytes = vec![0u8; m * groups_per_row * 72];
    let codebooks = [[-1.5f32, -0.25, 0.5, 1.75], [-2.0f32, -0.5, 0.25, 1.5]];
    let mut cpu = vec![0.0f32; m];

    for row in 0..m {
        for group in 0..groups_per_row {
            let codebook = codebooks[(row + group) & 1];
            let offset = (row * groups_per_row + group) * 72;
            for (i, value) in codebook.into_iter().enumerate() {
                bytes[offset + i * 2..offset + i * 2 + 2]
                    .copy_from_slice(&f32_to_f16(value).to_le_bytes());
            }
            for j in 0..256 {
                let index = ((row * 3 + group * 5 + j * 7) & 3) as u8;
                put_bits(&mut bytes[offset + 8..offset + 72], j * 2, 2, index);
                cpu[row] += codebook[index as usize] * x_rot[group * 256 + j];
            }
        }
    }

    let a = gpu.upload_raw(&bytes, &[bytes.len()]).unwrap();
    let x_gpu = gpu.upload_f32(&x, &[k]).unwrap();
    let y = gpu.zeros(&[m], DType::F32).unwrap();
    let scratch = gpu.zeros(&[k], DType::F32).unwrap();
    gpu.gemv_mq2g256_lloyd_with_rotate(&a, &x_gpu, &y, &scratch, m, k)
        .unwrap();
    compare("MQ2-G256-Lloyd", k, &cpu, &download(gpu, &y, m))
}

fn put_bits(payload: &mut [u8], bit: usize, bits: usize, value: u8) {
    let byte = bit / 8;
    let shift = bit % 8;
    let packed = (value as u16) << shift;
    payload[byte] |= packed as u8;
    if shift + bits > 8 {
        payload[byte + 1] |= (packed >> 8) as u8;
    }
}

fn get_bits(payload: &[u8], bit: usize, bits: usize) -> u8 {
    let byte = bit / 8;
    let shift = bit % 8;
    let high = payload.get(byte + 1).copied().unwrap_or(0) as u16;
    let packed = payload[byte] as u16 | high << 8;
    ((packed >> shift) & ((1u16 << bits) - 1)) as u8
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn download(gpu: &Gpu, tensor: &rdna_compute::GpuTensor, len: usize) -> Vec<f32> {
    let mut output = vec![0.0f32; len];
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(output.as_mut_ptr().cast::<u8>(), len * size_of::<f32>())
    };
    gpu.hip.memcpy_dtoh(bytes, &tensor.buf).unwrap();
    output
}

fn compare(name: &str, k: usize, cpu: &[f32], gpu: &[f32]) -> bool {
    let max_error = cpu
        .iter()
        .zip(gpu)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    let ok = max_error <= 2.0e-3;
    eprintln!(
        "{name:<18} K={k:<4} max_err={max_error:.6e} {}",
        if ok { "PASS" } else { "FAIL" }
    );
    ok
}

fn rotate_x(x: &[f32]) -> Vec<f32> {
    assert_eq!(x.len() % 256, 0);
    let signs1 = signs(42);
    let signs2 = signs(1042);
    let mut output = x.to_vec();
    for group in output.chunks_mut(256) {
        for i in 0..256 {
            group[i] *= signs1[i];
        }
        let mut stride = 1;
        while stride < 256 {
            for base in (0..256).step_by(stride * 2) {
                for j in 0..stride {
                    let a = group[base + j];
                    let b = group[base + j + stride];
                    group[base + j] = a + b;
                    group[base + j + stride] = a - b;
                }
            }
            stride *= 2;
        }
        for i in 0..256 {
            group[i] *= 0.0625 * signs2[i];
        }
    }
    output
}

fn signs(seed: u32) -> [f32; 256] {
    let mut state = seed;
    std::array::from_fn(|_| {
        state = state.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        if (state >> 16) & 1 == 1 {
            1.0
        } else {
            -1.0
        }
    })
}

fn f32_to_f16(value: f32) -> u16 {
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
    sign | ((exponent as u16) << 10) | ((mantissa >> 13) as u16)
}
