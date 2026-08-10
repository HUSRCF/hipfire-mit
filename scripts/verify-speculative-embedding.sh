#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Keep mask-based speculative input embedding batched and parity-covered.

set -euo pipefail
cd "$(dirname "$0")/.."

kernel=kernels/src/embedding_q8_batched.hip
dispatch=crates/rdna-compute/src/dispatch.rs
spec=crates/hipfire-arch-qwen35/src/speculative.rs
parity=crates/rdna-compute/examples/test_embedding_q8_seed_repeat.rs

for contract in \
    'void embedding_q8_seed_repeat(' \
    'pub fn embedding_lookup_q8_seed_repeat(' \
    'fn embed_dflash_seed_mask_block(' \
    'Q8 seed-repeat embedding parity PASS'
do
    if ! rg -Fq "$contract" "$kernel" "$dispatch" "$spec" "$parity"; then
        echo "speculative embedding audit: missing contract: $contract" >&2
        exit 1
    fi
done

call_count="$(rg -c 'embed_dflash_seed_mask_block\(' "$spec")"
if [ "$call_count" -lt 4 ]; then
    echo "speculative embedding audit: not all DFlash/DDTree draft paths share the batched helper" >&2
    exit 1
fi

if rg -q 'for \(i, &tok\) in block\.iter\(\)\.enumerate\(\)' "$spec"; then
    echo "speculative embedding audit: serial block embedding loop reintroduced" >&2
    exit 1
fi

if ! rg -Fq 'Q8 speculative seed-repeat embedding' scripts/quant-parity-gate.sh; then
    echo "speculative embedding audit: GPU parity case missing from mandatory battery" >&2
    exit 1
fi
if ! rg -Fq 'mean_draft_us=' scripts/coherence-gate-dflash.sh; then
    echo "speculative embedding audit: phase-performance evidence missing" >&2
    exit 1
fi

echo "speculative embedding audit: PASS (3 draft paths, exact GPU parity, phase evidence)"
