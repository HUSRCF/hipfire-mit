#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

./scripts/verify-kv-cache-footprint.sh
cargo test --locked -p hipfire-runtime --lib \
    llama::tests::packed_kv_footprint
cargo run --locked -q -p hipfire-runtime --example kv_cache_footprint >/dev/null

echo "test-kv-cache-footprint: PASS"
