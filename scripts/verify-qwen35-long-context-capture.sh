#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Audit Qwen3.5 long-context Q8 captured-prefill capability.

set -euo pipefail
cd "$(dirname "$0")/.."

source_file=crates/hipfire-arch-qwen35/src/qwen35.rs
probe=crates/hipfire-runtime/examples/profile_attention_phases.rs

if rg -q 'Q8 KV with.*physical_cap|hip\.malloc \+ memcpy_htod inside the' "$source_file"; then
    echo "Qwen3.5 long-context capture audit: stale Q8 capture refusal remains" >&2
    exit 1
fi

for contract in \
    'capture_prefill_probe' \
    'forward_prefill_batch_single_chunk_captured(' \
    'gpu.begin_graph_capture()' \
    'gpu.end_graph_capture()' \
    'gpu.graph_launch()' \
    'captured_prefill_blobs'
do
    rg -Fq "$contract" "$probe" || {
        echo "Qwen3.5 long-context capture audit: missing probe contract: $contract" >&2
        exit 1
    }
done

rg -Fq 'gpu.memcpy_dtod_at_auto(' "$source_file" || {
    echo "Qwen3.5 long-context capture audit: final hidden copy is not stream-aware" >&2
    exit 1
}

echo "Qwen3.5 long-context capture audit: PASS (explicit capture, launch, and record)"
