#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

./scripts/verify-architecture-adapters.sh >/dev/null

for expected in \
    'assert_eq!(adapter_family(0), Ok(AdapterFamily::Llama))' \
    'assert_eq!(adapter_family(1), Ok(AdapterFamily::Llama))' \
    'assert_eq!(adapter_family(5), Ok(AdapterFamily::Qwen35))' \
    'assert_eq!(adapter_family(6), Ok(AdapterFamily::Qwen35))' \
    'adapter_family(0xFF).unwrap_err()'
do
    if ! rg -Fq "$expected" crates/hipfire-runtime/examples/daemon.rs; then
        echo "architecture adapter self-test: missing route assertion: $expected" >&2
        exit 1
    fi
done

echo "test-architecture-adapters: PASS"
