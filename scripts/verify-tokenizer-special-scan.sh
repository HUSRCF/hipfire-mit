#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Ensure special-token discovery stays single-pass and parity-covered.

set -euo pipefail
cd "$(dirname "$0")/.."

source_file=crates/hipfire-runtime/src/tokenizer.rs
benchmark=crates/hipfire-runtime/examples/bench_tokenizer_encode.rs

for contract in \
    'special_token_initial_index: [Vec<usize>; 256]' \
    'fn next_special_match(&self, text: &str)' \
    'text.as_bytes().iter().enumerate()' \
    'indexed_special_scan_is_byte_identical_to_linear_reference'
do
    if ! rg -Fq "$contract" "$source_file"; then
        echo "tokenizer special-scan audit: missing contract: $contract" >&2
        exit 1
    fi
done

# The reference implementation below #[cfg(test)] intentionally retains the
# old exhaustive scan. Only reject that shape in production code.
production="$(sed '/^#\[cfg(test)\]/,$d' "$source_file")"
if rg -q 'remaining\.find\(|for \(token, _\) in &self\.special_tokens' <<<"$production"; then
    echo "tokenizer special-scan audit: exhaustive production scan reintroduced" >&2
    exit 1
fi

if [ ! -f "$benchmark" ] || ! rg -Fq 'median_ns=' "$benchmark"; then
    echo "tokenizer special-scan audit: reproducible microbenchmark missing" >&2
    exit 1
fi

echo "tokenizer special-scan audit: PASS (single-pass, reference parity, benchmark)"
