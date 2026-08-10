#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
cd "$repo"

./scripts/verify-hybrid-kv-allocation.sh
cargo check -p hipfire-runtime --example daemon --features deltanet --locked
