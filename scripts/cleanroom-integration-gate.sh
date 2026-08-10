#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Reproducible composition gate for independently developed clean-room batches.
#
# The full mode deliberately runs every currently available single-GPU
# correctness/quality gate plus 3-5 fresh speed processes. Each child GPU gate
# owns scripts/gpu-lock.sh, so GPU work remains serialized. Generated-language
# reports still require manual review before the integration result is accepted.
#
# Usage: ./scripts/cleanroom-integration-gate.sh [options]
#   --cpu-only        Run only source, workspace, audit, and license checks.
#   --speed-runs N    Run 3, 4, or 5 fresh speed processes (default: 3).
#   --out DIR         Store logs and the evidence manifest in DIR.
#   --print-plan      Print the ordered step names without executing them.
#   --self-check      Validate this gate and all delegated gate entry points.
#   -h, --help        Show this help text.

set -uo pipefail
cd "$(dirname "$0")/.." || exit 2

MODE="full"
SPEED_RUNS=3
OUT_DIR=""
ACTION="run"

CPU_STEPS=(
    source-diff-check
    workspace-tests
    workspace-examples
    bind-thread-audit
    architecture-adapter-audit
    hfq-consumer-shape-audit
    tokenizer-special-scan-audit
    speculative-embedding-audit
    greedy-batched-verify-audit
    kv-cache-footprint-audit
    hybrid-kv-allocation-audit
    long-context-q8-position-audit
    long-context-q8-record-audit
    qwen35-long-context-capture-audit
    generation-semantics-audit
    agentic-detector-self-check
    cleanroom-license
)
GPU_STEPS=(
    quant-parity
    coherence-standard
    coherence-dflash
    agentic-fast
)
REQUIRED_SCRIPTS=(
    scripts/agentic-gate.sh
    scripts/cleanroom-gate.sh
    scripts/coherence-gate-dflash.sh
    scripts/coherence-gate.sh
    scripts/quant-parity-gate.sh
    scripts/speed-gate.sh
    scripts/verify-architecture-adapters.sh
    scripts/verify-hfq-consumer-shapes.sh
    scripts/verify-tokenizer-special-scan.sh
    scripts/verify-speculative-embedding.sh
    scripts/verify-greedy-batched-verify.sh
    scripts/verify-kv-cache-footprint.sh
    scripts/verify-hybrid-kv-allocation.sh
    scripts/verify-long-context-q8-position-reuse.sh
    scripts/verify-long-context-q8-record.sh
    scripts/verify-qwen35-long-context-capture.sh
    scripts/verify-generation-semantics.sh
    scripts/verify-bind-thread.sh
)

usage() {
    sed -n '10,16p' "$0" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
    case "$1" in
        --cpu-only) MODE="cpu-only" ;;
        --speed-runs)
            [ $# -ge 2 ] || { echo "--speed-runs requires a value" >&2; exit 2; }
            SPEED_RUNS="$2"
            shift
            ;;
        --out)
            [ $# -ge 2 ] || { echo "--out requires a directory" >&2; exit 2; }
            OUT_DIR="$2"
            shift
            ;;
        --print-plan) ACTION="print-plan" ;;
        --self-check) ACTION="self-check" ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

case "$SPEED_RUNS" in
    3|4|5) ;;
    *) echo "--speed-runs must be 3, 4, or 5" >&2; exit 2 ;;
esac

print_plan() {
    local step run
    for step in "${CPU_STEPS[@]}"; do
        echo "$step"
    done
    if [ "$MODE" = "full" ]; then
        for step in "${GPU_STEPS[@]}"; do
            echo "$step"
        done
        for ((run = 1; run <= SPEED_RUNS; run++)); do
            echo "speed-fast-$run"
        done
    fi
}

if [ "$ACTION" = "print-plan" ]; then
    print_plan
    exit 0
fi

if [ "$ACTION" = "self-check" ]; then
    for script in "${REQUIRED_SCRIPTS[@]}"; do
        if [ ! -x "$script" ]; then
            echo "cleanroom-integration-gate: missing or non-executable $script" >&2
            exit 1
        fi
    done
    bash -n "$0" || exit 1
    echo "cleanroom-integration-gate: self-check PASS"
    exit 0
fi

if [ "$MODE" = "full" ]; then
    DEVICE="${HIPFIRE_INTEGRATION_DEVICE:-0}"
    if [ "${ROCR_VISIBLE_DEVICES:-}" != "$DEVICE" ] \
        || [ "${HIP_VISIBLE_DEVICES:-}" != "$DEVICE" ]; then
        echo "cleanroom-integration-gate: full mode requires both visibility variables to equal $DEVICE" >&2
        echo "  ROCR_VISIBLE_DEVICES=${ROCR_VISIBLE_DEVICES:-unset}" >&2
        echo "  HIP_VISIBLE_DEVICES=${HIP_VISIBLE_DEVICES:-unset}" >&2
        exit 2
    fi
fi

if [ -z "$OUT_DIR" ]; then
    OUT_DIR="/tmp/hipfire-integration-$(date +%Y%m%d-%H%M%S)-$$"
fi
mkdir -p "$OUT_DIR" || exit 2
SUMMARY="$OUT_DIR/summary.md"

STEP_NAMES=()
STEP_STATUS=()
STEP_LOGS=()
OVERALL="PASS"

write_summary() {
    local i dirty diff_hash
    dirty="$(git status --porcelain 2>/dev/null || true)"
    diff_hash="$(git diff --no-ext-diff --binary HEAD 2>/dev/null | sha256sum | awk '{print $1}')"
    {
        echo "<!-- SPDX-License-Identifier: MIT -->"
        echo "# Clean-room integration gate"
        echo
        echo "- head: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
        echo "- branch: $(git branch --show-current 2>/dev/null || echo unknown)"
        echo "- date: $(date -Iseconds)"
        echo "- mode: $MODE"
        echo "- diff_sha256: ${diff_hash:-unknown}"
        echo "- ROCR_VISIBLE_DEVICES: ${ROCR_VISIBLE_DEVICES:-unset}"
        echo "- HIP_VISIBLE_DEVICES: ${HIP_VISIBLE_DEVICES:-unset}"
        echo "- machine_result: $OVERALL"
        if [ "$MODE" = "full" ]; then
            echo "- generated_output_review: **REQUIRED**"
        else
            echo "- generated_output_review: not applicable"
        fi
        if [ -n "$dirty" ]; then
            echo "- worktree: dirty candidate"
        else
            echo "- worktree: clean"
        fi
        echo
        echo "| Step | Status | Log |"
        echo "|---|---|---|"
        for ((i = 0; i < ${#STEP_NAMES[@]}; i++)); do
            echo "| ${STEP_NAMES[$i]} | ${STEP_STATUS[$i]} | ${STEP_LOGS[$i]} |"
        done
        if [ "$MODE" = "full" ]; then
            echo
            echo "## Generated-output reports"
            echo
            echo "- quant parity: $OUT_DIR/quant-parity.md"
            echo "- standard coherence: $OUT_DIR/coherence.md"
            echo "- DFlash/DDTree coherence: $OUT_DIR/coherence-dflash.md"
            echo "- agentic structure: $OUT_DIR/agentic.md"
            echo
            echo "Read every available coherence and agentic output before accepting this result."
        fi
    } > "$SUMMARY"
}
trap write_summary EXIT

execute_step() {
    local step="$1"
    case "$step" in
        source-diff-check) git diff --check HEAD ;;
        workspace-tests) cargo test --workspace --locked --all-targets ;;
        workspace-examples) cargo check --workspace --locked --examples ;;
        bind-thread-audit) ./scripts/verify-bind-thread.sh ;;
        architecture-adapter-audit) ./scripts/verify-architecture-adapters.sh ;;
        hfq-consumer-shape-audit) ./scripts/verify-hfq-consumer-shapes.sh ;;
        tokenizer-special-scan-audit) ./scripts/verify-tokenizer-special-scan.sh ;;
        speculative-embedding-audit) ./scripts/verify-speculative-embedding.sh ;;
        greedy-batched-verify-audit) ./scripts/verify-greedy-batched-verify.sh ;;
        kv-cache-footprint-audit) ./scripts/verify-kv-cache-footprint.sh ;;
        hybrid-kv-allocation-audit) ./scripts/verify-hybrid-kv-allocation.sh ;;
        long-context-q8-position-audit) ./scripts/verify-long-context-q8-position-reuse.sh ;;
        long-context-q8-record-audit) ./scripts/verify-long-context-q8-record.sh ;;
        qwen35-long-context-capture-audit) ./scripts/verify-qwen35-long-context-capture.sh ;;
        generation-semantics-audit) ./scripts/verify-generation-semantics.sh ;;
        agentic-detector-self-check) ./scripts/agentic-gate.sh --self-check ;;
        cleanroom-license) ./scripts/cleanroom-gate.sh ;;
        quant-parity)
            env HIPFIRE_QUANT_PARITY_OUT="$OUT_DIR/quant-parity.md" \
                ./scripts/quant-parity-gate.sh
            ;;
        coherence-standard)
            env HIPFIRE_SKIP_PFLASH_GATE=1 \
                HIPFIRE_COHERENCE_OUT="$OUT_DIR/coherence.md" \
                ./scripts/coherence-gate.sh
            ;;
        coherence-dflash)
            env HIPFIRE_COHERENCE_OUT="$OUT_DIR/coherence-dflash.md" \
                ./scripts/coherence-gate-dflash.sh
            ;;
        agentic-fast)
            env HIPFIRE_AGENTIC_GATE_OUT="$OUT_DIR/agentic.md" \
                ./scripts/agentic-gate.sh --fast
            ;;
        speed-fast-*) ./scripts/speed-gate.sh --fast ;;
        *) echo "unknown integration step: $step" >&2; return 2 ;;
    esac
}

run_step() {
    local step="$1" log status
    log="$OUT_DIR/$step.log"
    echo
    echo "=== integration: $step ==="
    execute_step "$step" 2>&1 | tee "$log"
    status=${PIPESTATUS[0]}
    STEP_NAMES+=("$step")
    STEP_LOGS+=("$log")
    if [ "$status" -eq 0 ]; then
        STEP_STATUS+=("PASS")
        write_summary
        return 0
    fi
    STEP_STATUS+=("FAIL ($status)")
    OVERALL="FAIL"
    write_summary
    echo "cleanroom-integration-gate: $step failed (exit=$status)" >&2
    return "$status"
}

for step in "${CPU_STEPS[@]}"; do
    run_step "$step" || exit $?
done

if [ "$MODE" = "full" ]; then
    for step in "${GPU_STEPS[@]}"; do
        run_step "$step" || exit $?
    done
    for ((run = 1; run <= SPEED_RUNS; run++)); do
        run_step "speed-fast-$run" || exit $?
    done
fi

write_summary
trap - EXIT
echo
echo "cleanroom-integration-gate: machine checks PASS"
echo "summary: $SUMMARY"
if [ "$MODE" = "full" ]; then
    echo "manual review is still required for generated outputs"
fi
