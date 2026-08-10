#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

./scripts/verify-speculative-embedding.sh
bash -n scripts/coherence-gate-dflash.sh scripts/quant-parity-gate.sh

echo "test-speculative-embedding: PASS"
