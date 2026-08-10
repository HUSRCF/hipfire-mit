#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Ensure model loaders bind file-declared HFQ shapes to consumer dimensions.

set -euo pipefail
cd "$(dirname "$0")/.."

hfq=crates/hipfire-runtime/src/hfq.rs
qwen=crates/hipfire-arch-qwen35/src/qwen35.rs
dflash=crates/hipfire-runtime/src/dflash.rs

for contract in \
    'pub fn expect_shape(&self, expected: &[usize])' \
    'pub fn expect_numel(&self, expected: usize)'
do
    if ! rg -Fq "$contract" "$hfq"; then
        echo "HFQ consumer-shape audit: missing metadata contract: $contract" >&2
        exit 1
    fi
done

if ! rg -Fq 'fn load_weight_tensor_raw(gpu: &Gpu, info: &HfqTensorInfo' "$qwen"; then
    echo "HFQ consumer-shape audit: Qwen raw loader does not require tensor metadata" >&2
    exit 1
fi
if rg -n 'load_weight_tensor_raw\([^,]+, [^,]*quant_type,' "$qwen"; then
    echo "HFQ consumer-shape audit: Qwen call site bypasses tensor metadata" >&2
    exit 1
fi

for source in "$hfq" "$qwen" "$dflash"; do
    if ! rg -Fq 'expect_shape(&[m, k])?' "$source"; then
        echo "HFQ consumer-shape audit: $source matrix loader lacks exact shape check" >&2
        exit 1
    fi
done

if ! rg -Fq 'consumer_shape_contract_rejects_config_mismatch' "$hfq"; then
    echo "HFQ consumer-shape audit: missing negative shape-contract test" >&2
    exit 1
fi

echo "HFQ consumer-shape audit: PASS (LLaMA, Qwen3.5, DFlash)"
