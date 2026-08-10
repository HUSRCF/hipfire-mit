#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Ensure every production adapter validates its family before parsing config.

set -euo pipefail
cd "$(dirname "$0")/.."

for source_file in \
    crates/hipfire-arch-qwen35/src/arch.rs \
    crates/hipfire-arch-llama/src/arch.rs \
    crates/hipfire-arch-qwen35-vl/src/arch.rs
do
    body="$(sed -n '/fn config_from_hfq/,/fn load_weights/p' "$source_file")"
    if ! grep -Fq 'Self::supports_arch_id(hfq.arch_id)' <<<"$body"; then
        echo "adapter config-family audit: missing fail-closed check in $source_file" >&2
        exit 1
    fi
    if ! grep -Fq 'unsupported HFQ arch_id=' <<<"$body"; then
        echo "adapter config-family audit: missing diagnostic in $source_file" >&2
        exit 1
    fi
done

echo "adapter config-family audit: PASS (3 adapters fail closed before parsing)"
