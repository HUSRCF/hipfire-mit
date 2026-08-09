#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Static cross-platform guard for clean-room install/update provenance.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
fail() { echo "FAIL [install-provenance]: $*" >&2; exit 1; }

operational=(
    "$repo_root/scripts/install.sh"
    "$repo_root/scripts/install.ps1"
    "$repo_root/scripts/force-update.ps1"
    "$repo_root/scripts/amd_quickdeploy.sh"
    "$repo_root/cli/index.ts"
    "$repo_root/nix/package.nix"
    "$repo_root/nix/module.nix"
    "$repo_root/docs/GETTING_STARTED.md"
    "$repo_root/docs/NIXOS.md"
)

if rg -n 'Kaden-Schutt/hipfire|origin/master|github:Kaden-Schutt' "${operational[@]}"; then
    fail "an operational install path still names the post-boundary upstream"
fi

for path in \
    "$repo_root/scripts/install.sh" \
    "$repo_root/scripts/install.ps1" \
    "$repo_root/scripts/amd_quickdeploy.sh" \
    "$repo_root/cli/index.ts" \
    "$repo_root/nix/package.nix"; do
    rg -q -- '--locked' "$path" \
        || fail "lockfile enforcement missing from ${path#"$repo_root/"}"
done

for path in "$repo_root/scripts/install.sh" "$repo_root/scripts/install.ps1"; do
    rg -q 'HUSRCF/hipfire-mit' "$path" \
        || fail "clean-room default missing from ${path#"$repo_root/"}"
    rg -q 'HIPFIRE_INSTALL_REF' "$path" \
        || fail "explicit ref override missing from ${path#"$repo_root/"}"
    rg -q 'FETCH_HEAD' "$path" \
        || fail "immutable fetched-ref resolution missing from ${path#"$repo_root/"}"
    rg -q 'install-source.txt' "$path" \
        || fail "provenance record missing from ${path#"$repo_root/"}"
    rg -q 'checkout.*--detach.*--force' "$path" \
        || fail "immutable detached checkout missing from ${path#"$repo_root/"}"
done

if rg -q 'releases/latest|releases/download/hip-runtime' "$repo_root/scripts/install.ps1"; then
    fail "Windows installer still accepts mutable project release artifacts"
fi

rg -q 'default = "HUSRCF"' "$repo_root/nix/module.nix" \
    || fail "NixOS module owner does not default to the clean-room fork"
rg -q 'default = "hipfire-mit"' "$repo_root/nix/module.nix" \
    || fail "NixOS module repository does not default to the clean-room fork"

echo "PASS [install-provenance]: operational sources are clean-room pinned and lockfile-enforced"
