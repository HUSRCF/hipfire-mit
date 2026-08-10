// SPDX-License-Identifier: MIT
//! Emit a deterministic packed-KV capacity record without allocating a GPU.

use hipfire_runtime::llama::{packed_kv_footprint, PackedKvFormat};

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
    println!(
        "{}",
        serde_json::to_string_pretty(&records).expect("serialize footprint records")
    );
}
