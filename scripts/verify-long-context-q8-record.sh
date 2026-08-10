#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Keep the long-context Q8 evidence schema tied to the production benchmark.

set -euo pipefail
cd "$(dirname "$0")/.."

example=crates/hipfire-runtime/examples/profile_attention_phases.rs

for contract in \
    'schema: "hipfire.long_context_q8.v1"' \
    'prefill_tokens: prefill_len' \
    'sequence_length: seq_len' \
    'reference_attention_us: v1_us' \
    'flash_attention_us: flash_us' \
    'flash_speedup: v1_us / flash_us' \
    'max_abs_delta: max_abs' \
    'std::fs::write(&path'
do
    rg -Fq "$contract" "$example" || {
        echo "long-context Q8 record audit: missing contract: $contract" >&2
        exit 1
    }
done

echo "long-context Q8 record audit: PASS (timing, speedup, and parity schema)"
