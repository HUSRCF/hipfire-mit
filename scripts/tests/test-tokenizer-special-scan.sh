#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

./scripts/verify-tokenizer-special-scan.sh
cargo test --quiet --locked -p hipfire-runtime --lib \
    tokenizer::bpe_tests::indexed_special_scan_is_byte_identical_to_linear_reference \
    -- --exact

echo "test-tokenizer-special-scan: PASS"
