#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

daemon=crates/hipfire-runtime/examples/daemon.rs
runtime=crates/hipfire-runtime/src/llama.rs

for contract in \
    'let is_kv_layer: Vec<bool> = config.layer_types.iter()' \
    'new_gpu_q8_capped_filtered(gpu, &is_kv_layer' \
    'new_gpu_asym3_capped_filtered(gpu, &is_kv_layer'
do
    if ! grep -Fq "$contract" "$daemon"; then
        echo "hybrid KV allocation audit: missing daemon contract: $contract" >&2
        exit 1
    fi
done

for contract in \
    'pub fn new_gpu_q8_capped_filtered(' \
    'pub fn new_gpu_asym3_capped_filtered(' \
    'Self::alloc_k_v_filtered('
do
    if ! grep -Fq "$contract" "$runtime"; then
        echo "hybrid KV allocation audit: missing runtime contract: $contract" >&2
        exit 1
    fi
done

q8_arm=$(sed -n '/"q8" => {/,/^[[:space:]]*}/p' "$daemon")
asym3_arm=$(sed -n '/"asym3" | "turbo3"/,/^[[:space:]]*}/p' "$daemon")
if printf '%s\n%s\n' "$q8_arm" "$asym3_arm" | grep -Eq 'new_gpu_(q8|asym3)_capped\('; then
    echo "hybrid KV allocation audit: unfiltered primary constructor reintroduced" >&2
    exit 1
fi

echo "hybrid KV allocation audit: PASS (Q8 + default Asym3 filtered by FA layers)"
