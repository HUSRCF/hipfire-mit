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

llama_model=crates/hipfire-runtime/src/llama.rs
llama_hfq=crates/hipfire-runtime/src/hfq.rs
qwen_model=crates/hipfire-arch-qwen35/src/qwen35.rs

if ! rg -Fq 'ModelArch::from_model_type(arch_str)?' "$llama_model" "$llama_hfq"; then
    echo "adapter config-family audit: GGUF/HFQ parsers bypass the shared model-type contract" >&2
    exit 1
fi
if ! rg -Fq 'arch.matches_hfq_arch_id(hfq.arch_id)' "$llama_hfq"; then
    echo "adapter config-family audit: LLaMA HFQ header/config pairing is unchecked" >&2
    exit 1
fi
if rg -n "unknown architecture.*attempting LLaMA|_ => ModelArch::Llama" "$llama_model" "$llama_hfq"; then
    echo "adapter config-family audit: unknown model types still fall back to LLaMA" >&2
    exit 1
fi
if ! rg -Fq 'variant_matches_config(hfq.arch_id, num_experts > 0)' "$qwen_model"; then
    echo "adapter config-family audit: Qwen dense/MoE header/config pairing is unchecked" >&2
    exit 1
fi

echo "adapter config-family audit: PASS (family and variant metadata fail closed)"
