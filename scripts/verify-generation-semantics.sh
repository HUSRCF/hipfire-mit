#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Keep user-facing decode loops on the shared generation-stop contract.

set -euo pipefail
cd "$(dirname "$0")/.."

entry_points=(
    crates/hipfire-runtime/examples/daemon.rs
    crates/hipfire-runtime/examples/infer.rs
    crates/hipfire-runtime/examples/infer_hfq.rs
    crates/hipfire-runtime/examples/infer_qwen3.rs
    crates/hipfire-runtime/examples/infer_qwen35.rs
    crates/hipfire-runtime/examples/infer_vl.rs
    crates/hipfire-runtime/examples/run.rs
)

for source in "${entry_points[@]}"; do
    if ! rg -q 'is_generation_stop' "$source"; then
        echo "generation semantics audit: $source bypasses is_generation_stop" >&2
        exit 1
    fi
done

if rg -n '(next_token|tok) == [^;]*(eos_token|eos_id)' "${entry_points[@]}"; then
    echo "generation semantics audit: bare EOS comparison remains in a user-facing loop" >&2
    exit 1
fi

echo "generation semantics audit: PASS (${#entry_points[@]} entry points)"
