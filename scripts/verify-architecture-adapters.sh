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

daemon=crates/hipfire-runtime/examples/daemon.rs
if ! rg -q 'adapter_family\(hfq\.arch_id\)\?' "$daemon"; then
    echo "architecture adapter audit: daemon load path bypasses adapter selection" >&2
    exit 1
fi

if rg -n 'if hfq\.arch_id (== 5 \|\||!= 5 &&)' "$daemon"; then
    echo "architecture adapter audit: daemon duplicated Qwen family membership" >&2
    exit 1
fi

echo "architecture adapter audit: PASS (${#adapter_sources[@]} family adapters)"
