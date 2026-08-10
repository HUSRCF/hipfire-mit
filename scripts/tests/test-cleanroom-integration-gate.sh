#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

gate="./scripts/cleanroom-integration-gate.sh"
bash -n "$gate"
"$gate" --self-check >/dev/null

cpu_expected=$'source-diff-check\nworkspace-tests\nworkspace-examples\nbind-thread-audit\narchitecture-adapter-audit\nhfq-consumer-shape-audit\ntokenizer-special-scan-audit\nspeculative-embedding-audit\ngeneration-semantics-audit\nagentic-detector-self-check\ncleanroom-license'
cpu_actual="$($gate --cpu-only --print-plan)"
if [ "$cpu_actual" != "$cpu_expected" ]; then
    echo "test-cleanroom-integration-gate: CPU plan drift" >&2
    exit 1
fi

full_expected="$cpu_expected"$'\nquant-parity\ncoherence-standard\ncoherence-dflash\nagentic-fast\nspeed-fast-1\nspeed-fast-2\nspeed-fast-3\nspeed-fast-4\nspeed-fast-5'
full_actual="$($gate --speed-runs 5 --print-plan)"
if [ "$full_actual" != "$full_expected" ]; then
    echo "test-cleanroom-integration-gate: full plan drift" >&2
    exit 1
fi

if "$gate" --speed-runs 2 --print-plan >/dev/null 2>&1; then
    echo "test-cleanroom-integration-gate: accepted fewer than 3 speed runs" >&2
    exit 1
fi

echo "test-cleanroom-integration-gate: PASS"
