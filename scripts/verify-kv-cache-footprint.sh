#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Keep direction-row 53 capacity evidence derived from the allocation contract.

set -euo pipefail
cd "$(dirname "$0")/.."

source_file=crates/hipfire-runtime/src/llama.rs
example=crates/hipfire-runtime/examples/kv_cache_footprint.rs

for contract in \
    'pub enum PackedKvFormat {' \
    'pub struct PackedKvFootprint {' \
    'pub fn packed_kv_footprint(' \
    'pub struct HybridPackedKvFootprint {' \
    'pub fn hybrid_packed_kv_footprint(' \
    'let layout = PackedKvLayout::new(' \
    'packed_kv_footprint_records_real_rounded_allocations' \
    'PackedKvFormat::Asym3' \
    'qwen35_hybrid'
do
    if ! rg -Fq "$contract" "$source_file" "$example"; then
        echo "KV cache footprint audit: missing contract: $contract" >&2
        exit 1
    fi
done

base_format_count="$(sed -n '/let formats = \[/,/^    \];/p' "$example" | rg -o 'PackedKvFormat::(Q8|Asym2|Asym3|Asym4)' | wc -l)"
hybrid_format_count="$(sed -n '/let hybrid = \[/,/\.map/p' "$example" | rg -o 'PackedKvFormat::(Q8|Asym2|Asym3|Asym4)' | wc -l)"
if [ "$base_format_count" -ne 4 ] || [ "$hybrid_format_count" -ne 2 ]; then
    echo "KV cache footprint audit: canonical report must cover four packed formats and two hybrid records" >&2
    exit 1
fi

echo "KV cache footprint audit: PASS (shared checked layout, four packed formats, two hybrid records)"
