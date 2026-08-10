// SPDX-License-Identifier: MIT
//! Exact GPU parity for the Q8 seed-plus-repeated-token embedding launch.

use rdna_compute::{DType, Gpu};

fn main() {
    let mut gpu = Gpu::init().expect("GPU init");
    let vocab = 4usize;
    let dim = 64usize;
    let n = 7usize;
    let seed = 3u32;
    let repeat = 1u32;
    let bytes_per_row = (dim / 32) * 34;

    // Two Q8_0 blocks per row, scale=1.0 (IEEE fp16 0x3c00), with a
    // distinct deterministic signed-byte pattern for every vocabulary row.
    let mut table = vec![0u8; vocab * bytes_per_row];
    for row in 0..vocab {
        for block in 0..dim / 32 {
            let offset = row * bytes_per_row + block * 34;
            table[offset] = 0x00;
            table[offset + 1] = 0x3c;
            for within in 0..32 {
                let value = row as i8 * 17 + block as i8 * 5 + within as i8 - 31;
                table[offset + 2 + within] = value as u8;
            }
        }
    }

    let d_table = gpu.upload_raw(&table, &[table.len()]).expect("upload table");
    let reference = gpu.zeros(&[n * dim], DType::F32).expect("reference output");
    let candidate = gpu.zeros(&[n * dim], DType::F32).expect("candidate output");

    for row in 0..n {
        let token = if row == 0 { seed } else { repeat };
        let output_row = reference.sub_offset(row * dim, dim);
        gpu.embedding_lookup_q8(&d_table, &output_row, token, dim)
            .expect("single-row lookup");
    }
    gpu.embedding_lookup_q8_seed_repeat(&d_table, &candidate, seed, repeat, n, dim)
        .expect("seed-repeat lookup");

    let reference_host = gpu.download_f32(&reference).expect("download reference");
    let candidate_host = gpu.download_f32(&candidate).expect("download candidate");
    assert_eq!(candidate_host, reference_host, "seed-repeat embedding diverged");
    println!("Q8 seed-repeat embedding parity PASS: rows={n} dim={dim}");
}
