#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
cd "$repo"

./scripts/verify-greedy-batched-verify.sh
cargo test -p hipfire-arch-qwen35 greedy_plan --features deltanet --locked
cargo check -p hipfire-runtime --example run --features deltanet --locked
