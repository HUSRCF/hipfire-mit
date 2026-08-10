#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Keep both quantizer input formats on one fail-closed HFQ architecture map.

set -euo pipefail
cd "$(dirname "$0")/.."

source_file=crates/hipfire-quantize/src/main.rs

if ! rg -Fq 'struct HfqArchitecture' "$source_file"; then
    echo "quantizer architecture audit: shared contract is missing" >&2
    exit 1
fi
if [ "$(rg -c 'HfqArchitecture::from_source_name' "$source_file")" -lt 5 ]; then
    echo "quantizer architecture audit: input paths or tests bypass shared mapping" >&2
    exit 1
fi
if rg -n "unknown .*architecture.*(treating|tagging).*llama|arch_id == 6" "$source_file"; then
    echo "quantizer architecture audit: legacy fallback or variant hard-code remains" >&2
    exit 1
fi
if ! rg -Fq "unsupported model architecture 'gemma4'" "$source_file"; then
    echo "quantizer architecture audit: unknown-family fail-closed test is missing" >&2
    exit 1
fi

echo "quantizer architecture audit: PASS (GGUF + Safetensors share fail-closed mapping)"
