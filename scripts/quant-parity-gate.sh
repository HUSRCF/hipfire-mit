#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Deterministic CPU-reference/GPU-parity battery for registered quant paths.
#
# The cases use synthetic weights and activations, so this gate does not need
# model files. Every executable owns a numerical tolerance and exits non-zero
# when its GPU output disagrees with its independent CPU reference.

set -u
cd "$(dirname "$0")/.."

OUT="${HIPFIRE_QUANT_PARITY_OUT:-/tmp/quant-parity-$(date +%Y%m%d-%H%M%S).md}"
LOCK_SCRIPT="./scripts/gpu-lock.sh"
CASE_OUTPUT="$(mktemp /tmp/hipfire-quant-parity-case.XXXXXX)" || exit 2
LOCK_HELD=0

cleanup() {
    rm -f "$CASE_OUTPUT"
    if [ "$LOCK_HELD" -eq 1 ]; then
        gpu_release 2>/dev/null || true
    fi
}
trap cleanup EXIT

runtime_examples=(
    --example test_hfq4g256QA
    --example test_hfq6g256
    --example test_classic_quant_parity
    --example verify_mq_kernel
    --example test_gemv_hfq3g256_residual
    --example test_q8kvQA
)
compute_examples=(
    --example test_gemv_mq3g256_lloyd_tail
    --example test_gemv_hfp4g32
    --example test_gemv_mfp4g32
)

echo "quant-parity-gate: building deterministic parity anchors..."
if ! cargo build --quiet --release -p hipfire-runtime \
    "${runtime_examples[@]}" --features deltanet; then
    echo "quant-parity-gate: hipfire-runtime example build failed" >&2
    exit 2
fi
if ! cargo build --quiet --release -p rdna-compute \
    "${compute_examples[@]}"; then
    echo "quant-parity-gate: rdna-compute example build failed" >&2
    exit 2
fi

if [ ! -r "$LOCK_SCRIPT" ]; then
    echo "quant-parity-gate: missing GPU lock helper: $LOCK_SCRIPT" >&2
    exit 2
fi
# shellcheck disable=SC1090
. "$LOCK_SCRIPT"
gpu_acquire "quant-parity-gate" || exit 2
LOCK_HELD=1

{
    echo "# Quantization numerical parity battery"
    echo
    echo "- commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
    echo "- branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
    echo "- date: $(date -Iseconds)"
    echo "- ROCR_VISIBLE_DEVICES: ${ROCR_VISIBLE_DEVICES:-unset}"
    echo "- HIP_VISIBLE_DEVICES: ${HIP_VISIBLE_DEVICES:-unset}"
    echo
    echo "Every case compares GPU output with a deterministic CPU reference and"
    echo "hard-fails when its format-specific numerical tolerance is exceeded."
    echo
} > "$OUT"

failures=0
run_case() {
    local label="$1"
    local executable="$2"
    local status

    echo "== $label =="
    if "$executable" > "$CASE_OUTPUT" 2>&1; then
        status=0
    else
        status=$?
        failures=$((failures + 1))
    fi

    {
        echo "## $label"
        echo
        if [ "$status" -eq 0 ]; then
            echo "- status: **PASS**"
        else
            echo "- status: **FAIL** (exit=$status)"
        fi
        echo
        echo '```text'
        sed -n '1,240p' "$CASE_OUTPUT"
        echo '```'
        echo
    } >> "$OUT"
}

run_case "HFQ4-G256 GEMV, embedding, and MMQ residual" \
    ./target/release/examples/test_hfq4g256QA
run_case "HFQ6-G256 GEMV" \
    ./target/release/examples/test_hfq6g256
run_case "Classic Q4K, Q4F16-G32/G64, Q8_0, and Q8HFQ GEMV" \
    ./target/release/examples/test_classic_quant_parity
run_case "MQ6-G256, MQ3-G256, and MQ2-G256 rotated GEMV" \
    ./target/release/examples/verify_mq_kernel
run_case "HFQ3-G256 residual GEMV shapes" \
    ./target/release/examples/test_gemv_hfq3g256_residual
run_case "Q8 KV cache write and attention" \
    ./target/release/examples/test_q8kvQA
run_case "MQ3-G256-Lloyd GEMV tail groups" \
    ./target/release/examples/test_gemv_mq3g256_lloyd_tail
run_case "HFP4-G32 GEMV tail groups" \
    ./target/release/examples/test_gemv_hfp4g32
run_case "MFP4-G32 rotated GEMV tail groups" \
    ./target/release/examples/test_gemv_mfp4g32

echo
echo "quant parity report: $OUT"
if [ "$failures" -ne 0 ]; then
    echo "quant-parity-gate: $failures case(s) failed" >&2
    exit 1
fi
echo "quant-parity-gate: all 9 cases passed"
