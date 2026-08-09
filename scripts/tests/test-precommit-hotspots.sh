#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

hook=".githooks/pre-commit"
hotspot="$(awk -F"'" '/^HOTSPOT=/{print $2; exit}' "$hook")"
if [ -z "$hotspot" ]; then
    echo "test-precommit-hotspots: could not read HOTSPOT from $hook" >&2
    exit 1
fi

required_paths=(
    "crates/hipfire-runtime/examples/daemon.rs"
    "crates/hipfire-runtime/src/tool_call.rs"
    "scripts/agentic-gate.sh"
)

for path in "${required_paths[@]}"; do
    if ! grep -qE "$hotspot" <<< "$path"; then
        echo "test-precommit-hotspots: HOTSPOT misses $path" >&2
        exit 1
    fi
done

echo "test-precommit-hotspots: PASS (${#required_paths[@]} agentic paths)"
