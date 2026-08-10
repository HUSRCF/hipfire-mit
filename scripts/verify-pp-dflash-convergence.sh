#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Ensure incomplete pipeline-parallel DFlash cannot bypass the stable guard.

set -euo pipefail
cd "$(dirname "$0")/.."

daemon=crates/hipfire-runtime/examples/daemon.rs

if rg -n 'HIPFIRE_PP_DFLASH' "$daemon"; then
    echo "PP/DFlash convergence audit: experimental refusal bypass is reachable" >&2
    exit 1
fi
if ! rg -Fq 'pp_dflash_requires_single_gpu(pp, draft_path.is_some())' "$daemon"; then
    echo "PP/DFlash convergence audit: load boundary lacks fail-closed guard" >&2
    exit 1
fi
if ! rg -Fq 'fn pipeline_parallel_dflash_fails_closed()' "$daemon"; then
    echo "PP/DFlash convergence audit: guard truth-table test is missing" >&2
    exit 1
fi
if ! rg -Fq 'cross-device draft/target coordination is not implemented' "$daemon"; then
    echo "PP/DFlash convergence audit: operator diagnostic is not explicit" >&2
    exit 1
fi

echo "PP/DFlash convergence audit: PASS (pp>1 + draft fails closed)"
