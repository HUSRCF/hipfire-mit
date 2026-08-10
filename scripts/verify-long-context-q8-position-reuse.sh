#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Audit long-context Q8 prefill position reuse across plain and hybrid paths.

set -euo pipefail
cd "$(dirname "$0")/.."

plain=crates/hipfire-runtime/src/llama.rs
hybrid=crates/hipfire-arch-qwen35/src/qwen35.rs

view_count="$(rg -o 'let pos_b_view = pbs\.positions\.sub_offset\(b, 1\);' "$plain" "$hybrid" | wc -l)"
if [ "$view_count" -ne 3 ]; then
    echo "long-context Q8 position audit: expected three device-view fallbacks, found $view_count" >&2
    exit 1
fi

if rg -q 'pos_buf_tmp|Q8 KV with physical_cap .*hits the per-position long-context fallback' "$plain" "$hybrid"; then
    echo "long-context Q8 position audit: loop-local position allocation or stale capture rejection remains" >&2
    exit 1
fi

for source in "$plain" "$hybrid"; do
    rg -Fq '&pos_b_view.buf' "$source" || {
        echo "long-context Q8 position audit: missing device-view dispatch in $source" >&2
        exit 1
    }
done

echo "long-context Q8 position audit: PASS (3 fallbacks reuse uploaded device positions)"
