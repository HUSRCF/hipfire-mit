// SPDX-License-Identifier: MIT
//! Emit a deterministic packed-KV capacity record without allocating a GPU.

use hipfire_runtime::llama::{
    hybrid_packed_kv_footprint, packed_kv_footprint, HybridPackedKvFootprint,
    PackedKvFootprint, PackedKvFormat,
};
use serde::Serialize;

#[derive(Serialize)]
struct CapacityRecords {
    packed_formats: [PackedKvFootprint; 4],
    qwen35_hybrid: [HybridPackedKvFootprint; 2],
}

fn main() {
    let formats = [
        PackedKvFormat::Q8,
        PackedKvFormat::Asym2,
        PackedKvFormat::Asym3,
        PackedKvFormat::Asym4,
    ];
    let records = formats.map(|format| {
        packed_kv_footprint(format, 16, 4, 256, 65_536, 2048)
            .expect("canonical packed KV footprint")
    });
    let hybrid = [PackedKvFormat::Q8, PackedKvFormat::Asym3].map(|format| {
        hybrid_packed_kv_footprint(format, 64, 16, 4, 256, 65_536, 2048)
            .expect("canonical hybrid packed KV footprint")
    });
    let output = CapacityRecords {
        packed_formats: records,
        qwen35_hybrid: hybrid,
    };
    println!("{}", serde_json::to_string_pretty(&output).expect("serialize footprint records"));
}
