#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Ensure DFlash/DDTree terminators are classified before any user-visible emit.

set -euo pipefail
cd "$(dirname "$0")/.."

source_file=crates/hipfire-runtime/examples/daemon.rs
first_check=$(rg -n -m1 'let first_is_stop = tokenizer\.is_generation_stop' "$source_file" | cut -d: -f1)
first_emit=$(rg -n -m1 'streamed_tokens\.push\(first_token\)' "$source_file" | cut -d: -f1)
tail_check=$(rg -n -m1 'if tokenizer\.is_generation_stop\(tok, target\.config\.eos_token' "$source_file" | cut -d: -f1)
tail_emit=$(rg -n -m1 'streamed_tokens\.push\(tok\)' "$source_file" | cut -d: -f1)

if [[ -z "$first_check" || -z "$first_emit" || "$first_check" -ge "$first_emit" ]]; then
    echo "DFlash stop-order audit: first-token terminator check is missing or late" >&2
    exit 1
fi
if [[ -z "$tail_check" || -z "$tail_emit" || "$tail_check" -ge "$tail_emit" ]]; then
    echo "DFlash stop-order audit: committed-tail terminator check is missing or late" >&2
    exit 1
fi
if ! rg -q 'while !first_is_stop && generated < max_tokens' "$source_file"; then
    echo "DFlash stop-order audit: first-token stop does not bypass speculation" >&2
    exit 1
fi

echo "DFlash stop-order audit: PASS (first and batched terminators precede emit)"
