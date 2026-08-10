#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Ensure HFQ family ownership stays in architecture adapters.

set -euo pipefail
cd "$(dirname "$0")/.."

adapter_sources=(
    crates/hipfire-arch-llama/src/arch.rs
    crates/hipfire-arch-qwen35/src/arch.rs
    crates/hipfire-arch-qwen35-vl/src/arch.rs
)

for source in "${adapter_sources[@]}"; do
    if ! rg -q 'fn supports_arch_id\(' "$source"; then
        echo "architecture adapter audit: $source does not declare family ownership" >&2
        exit 1
    fi
done

for source in \
    crates/hipfire-arch-llama/src/arch.rs \
    crates/hipfire-arch-qwen35/src/arch.rs
do
    if ! rg -q 'fn protocol_label\(' "$source"; then
        echo "architecture adapter audit: $source does not own protocol labels" >&2
        exit 1
    fi
done

daemon=crates/hipfire-runtime/examples/daemon.rs
if ! rg -q 'adapter_family\(hfq\.arch_id\)\?' "$daemon"; then
    echo "architecture adapter audit: daemon load path bypasses adapter selection" >&2
    exit 1
fi

if rg -n 'if hfq\.arch_id (== 5 \|\||!= 5 &&)' "$daemon"; then
    echo "architecture adapter audit: daemon duplicated Qwen family membership" >&2
    exit 1
fi

label_body="$(sed -n '/fn model_arch_label/,/^}/p' "$daemon")"
if grep -Eq 'arch_id == 6|"qwen3_5_moe"' <<<"$label_body"; then
    echo "architecture adapter audit: daemon duplicated variant labels" >&2
    exit 1
fi

if ! rg -q '<Qwen35 as Architecture>::protocol_label\(arch_id\)' "$daemon"; then
    echo "architecture adapter audit: daemon bypasses adapter protocol labels" >&2
    exit 1
fi

echo "architecture adapter audit: PASS (${#adapter_sources[@]} family adapters)"
