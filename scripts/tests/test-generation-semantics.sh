#!/usr/bin/env bash
# SPDX-License-Identifier: MIT

set -euo pipefail
cd "$(dirname "$0")/../.."

./scripts/verify-generation-semantics.sh >/dev/null

tmp_dir="$(mktemp -d /tmp/hipfire-generation-semantics.XXXXXX)"
trap 'rm -rf "$tmp_dir"' EXIT

cp crates/hipfire-runtime/examples/infer.rs "$tmp_dir/infer.rs"
sed -i 's/tokenizer\.is_generation_stop(next_token, text_config\.eos_token, im_end_token)/next_token == text_config.eos_token/' "$tmp_dir/infer.rs"

if rg -q 'is_generation_stop' "$tmp_dir/infer.rs"; then
    echo "generation semantics self-test: failed to construct drift fixture" >&2
    exit 1
fi
if ! rg -q '(next_token|tok) == [^;]*(eos_token|eos_id)' "$tmp_dir/infer.rs"; then
    echo "generation semantics self-test: bare EOS detector missed fixture" >&2
    exit 1
fi

echo "test-generation-semantics: PASS"
