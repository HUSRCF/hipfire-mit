#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Keep the HFQ BF16 producer, wire registry, and vision consumer compatible.

set -euo pipefail
cd "$(dirname "$0")/.."

quantizer=crates/hipfire-quantize/src/main.rs
hfq=crates/hipfire-runtime/src/hfq.rs
vision=crates/hipfire-arch-qwen35-vl/src/qwen35_vl.rs

if ! rg -Fq 'BF16 = 16' "$quantizer"; then
    echo "vision BF16 audit: quantizer no longer emits the registered qt16 ID" >&2
    exit 1
fi
if ! rg -Fq '1 | 16 => Layout::Dense { bytes_per_element: 2 }' "$hfq"; then
    echo "vision BF16 audit: HFQ registry does not validate qt16 as dense 2-byte data" >&2
    exit 1
fi
if ! rg -Fq '16 => bf16_bytes_to_f32(data)' "$vision"; then
    echo "vision BF16 audit: F32 vision parameters do not consume qt16" >&2
    exit 1
fi
if ! rg -Fq 'let f16_bytes = bf16_bytes_to_f16(data);' "$vision"; then
    echo "vision BF16 audit: F16 vision kernels do not consume qt16" >&2
    exit 1
fi
if ! rg -Fq 'bf16_payload_widens_exactly_to_f32' "$vision" \
    || ! rg -Fq 'bf16_payload_converts_to_vision_kernel_f16' "$vision"; then
    echo "vision BF16 audit: conversion contract tests are missing" >&2
    exit 1
fi

echo "vision BF16 audit: PASS (producer, wire registry, F32/F16 consumers)"
