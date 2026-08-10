#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Keep both quantizer input formats on one fail-closed HFQ architecture map.

set -euo pipefail
cd "$(dirname "$0")/.."

source_file=crates/hipfire-quantize/src/main.rs
registry=crates/hipfire-format/src/lib.rs

if ! rg -Fq 'ModelArchitecture::from_model_type' "$source_file"; then
    echo "quantizer architecture audit: shared wire registry is bypassed" >&2
    exit 1
fi
if [ "$(rg -c 'hfq_architecture_from_source_name' "$source_file")" -lt 5 ]; then
    echo "quantizer architecture audit: input paths or tests bypass shared mapping" >&2
    exit 1
fi
if ! rg -Fq 'pub struct ModelArchitecture' "$registry"; then
    echo "quantizer architecture audit: shared model architecture registry is missing" >&2
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
if ! rg -Fq 'hfq_architecture_from_source_name("mistral")' "$source_file"; then
    echo "quantizer architecture audit: explicit Mistral family coverage is missing" >&2
    exit 1
fi

echo "quantizer architecture audit: PASS (producer + consumer share wire registry)"
