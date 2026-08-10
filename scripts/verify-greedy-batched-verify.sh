#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

spec=crates/hipfire-arch-qwen35/src/speculative.rs
runner=crates/hipfire-runtime/examples/run.rs

for contract in \
    'pub fn spec_step_greedy(' \
    'let verify_out = verify_dflash_block(' \
    'target_predictions.extend_from_slice(&verify_out.argmax_per_pos)' \
    'hidden_rb: &mut HiddenStateRingBuffer' \
    'verify_scratch: &VerifyScratch'
do
    if ! grep -Fq "$contract" "$spec"; then
        echo "greedy batched verify audit: missing contract: $contract" >&2
        exit 1
    fi
done

for contract in \
    'spec_hidden_rb = Some(HiddenStateRingBuffer::new(' \
    'spec_verify_scratch = Some(VerifyScratch::with_prefill(' \
    'hidden_rb, verify_scratch,'
do
    if ! grep -Fq "$contract" "$runner"; then
        echo "greedy batched verify audit: runner does not reuse: $contract" >&2
        exit 1
    fi
done

body=$(sed -n '/pub fn spec_step_greedy(/,/^\/\/ ═══/p' "$spec")
verify=$(printf '%s\n' "$body" | sed -n '/\/\/ Verification:/,/\/\/ Acceptance:/p')
if printf '%s\n' "$verify" | grep -Fq 'target.forward('; then
    echo "greedy batched verify audit: serial target verification reintroduced" >&2
    exit 1
fi

echo "greedy batched verify audit: PASS"
