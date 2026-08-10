#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

./scripts/verify-hfq-consumer-shapes.sh >/dev/null
cargo test -p hipfire-runtime \
    hfq::tests::consumer_shape_contract_rejects_config_mismatch --locked >/dev/null

echo "test-hfq-consumer-shapes: PASS"
