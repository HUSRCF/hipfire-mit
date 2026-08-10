#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Ensure experimental pipeline-parallel PFlash cannot bypass the stable guard.

set -euo pipefail
cd "$(dirname "$0")/.."

daemon=crates/hipfire-runtime/examples/daemon.rs
env_doc=docs/env-vars.md

if rg -n 'HIPFIRE_PP_PFLASH' "$daemon" "$env_doc"; then
    echo "PP/PFlash convergence audit: experimental refusal bypass is live" >&2
    exit 1
fi
if ! rg -Fq 'pp_pflash_requires_single_gpu(' "$daemon"; then
    echo "PP/PFlash convergence audit: load boundary lacks fail-closed guard" >&2
    exit 1
fi
if ! rg -Fq 'fn pipeline_parallel_pflash_fails_closed()' "$daemon"; then
    echo "PP/PFlash convergence audit: guard truth-table test is missing" >&2
    exit 1
fi
if ! rg -Fq 'pipeline-parallel PFlash has no validated resource and parity contract' "$daemon"; then
    echo "PP/PFlash convergence audit: operator diagnostic is not explicit" >&2
    exit 1
fi

echo "PP/PFlash convergence audit: PASS (pp>1 + PFlash fails closed)"
