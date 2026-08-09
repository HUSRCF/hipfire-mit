#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

base_ref="${1:-${CLEANROOM_BASE_REF:-mit-baseline-20260519}}"
failures=0

fail() {
    echo "cleanroom-gate: $*" >&2
    failures=$((failures + 1))
}

if ! git rev-parse --verify "${base_ref}^{commit}" >/dev/null 2>&1; then
    echo "cleanroom-gate: base ref '$base_ref' is not available" >&2
    exit 2
fi

if ! git merge-base --is-ancestor "$base_ref" HEAD; then
    fail "HEAD does not descend from $base_ref"
fi

if ! rg -q --fixed-strings "MIT License" LICENSE; then
    fail "root LICENSE is not the MIT License"
fi

invalid_packages="$(
    cargo metadata --format-version 1 --no-deps --locked |
        python3 -c '
import json
import sys

metadata = json.load(sys.stdin)
for package in metadata["packages"]:
    if package.get("license") != "MIT":
        print("{}: {!r}".format(package["name"], package.get("license")))
'
)"
if [[ -n "$invalid_packages" ]]; then
    fail "workspace packages without an effective MIT license:"
    printf '%s\n' "$invalid_packages" >&2
fi

while IFS= read -r path; do
    [[ -n "$path" && -f "$path" ]] || continue

    case "$path" in
        *.md)
            marker="<!-- SPDX-License-Identifier: MIT -->"
            ;;
        *.rs|*.hip|*.c|*.cc|*.cpp|*.h|*.hpp|*.ts|*.js)
            marker="// SPDX-License-Identifier: MIT"
            ;;
        *.sh|*.py|*.toml|*.yml|*.yaml)
            marker="# SPDX-License-Identifier: MIT"
            ;;
        *)
            continue
            ;;
    esac

    if ! head -n 6 "$path" | rg -q --fixed-strings "$marker"; then
        fail "$path is changed after $base_ref but lacks '$marker' near its start"
    fi
done < <(git diff --name-only --diff-filter=ACMRT "$base_ref" --)

if ((failures > 0)); then
    echo "cleanroom-gate: FAILED ($failures issue(s))" >&2
    exit 1
fi

echo "cleanroom-gate: PASS (base=$base_ref, outbound license=MIT)"
