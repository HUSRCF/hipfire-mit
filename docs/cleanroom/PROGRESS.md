<!-- SPDX-License-Identifier: MIT -->
# Clean-room progress ledger

Baseline: `3e4cd7d80783b37190759bb876699216b56319c6`

Local repository baseline tag: `mit-baseline-20260519`

Target repository: `https://github.com/HUSRCF/hipfire-mit.git`

This ledger tracks independently defined milestones. A direction-table row
is not marked complete merely because the MIT snapshot already contains a
related mechanism; it must have an explicit requirement, test, and evidence.

| Milestone | Direction rows | State | Independent deliverable |
|---|---:|---|---|
| M0: governance and reproducibility | 1-2 | complete | MIT/source gate, clean-room PR declaration, build and performance baselines |
| M1: speculative decode foundation | 3-8 | complete | shared greedy acceptance contract, semantic statistics, deterministic tests, and GPU0 correctness/throughput acceptance |
| M2: release and device execution | 10-18 | in progress | reproducible delivery and the device-execution foundation are complete; remaining quantization, maintenance, and integration directions are still open |
| M3: quantization and context paths | 19-39 | planned | format/quality tests and cache continuity/performance tests |

## M0 evidence

- Remote configured locally as `origin` at the target repository.
- Two `gfx1100` devices are visible through ROCm 7.14.
- The read-only local model library contains 9B and 27B target/draft pairs.
- `scripts/cleanroom-gate.sh` enforces effective workspace package licenses
  and MIT SPDX markers on files changed after the baseline tag.
- `cargo check --workspace --locked` passes on Rust 1.97.1. Existing compiler
  warnings remain and no new warning was introduced by M0.
- The committed RX 7900 XTX `gfx1100` floor is intentionally unchanged. The
  local device is a clock-limited 48GB Radeon Pro W7900, so its measured
  board-specific floor is recorded separately in
  `tests/speed-baselines/gfx1100-w7900.txt`.

## Performance baseline record

The M0 measurements and hashes are recorded in
`docs/cleanroom/PERFORMANCE_RECORD.md`. Use the same format for every later
baseline/candidate pair. Do not replace measured values with remembered or
upstream-published values.

## M1 evidence

The direction table exposes only the high-level goal of reducing serial
target-model calls through low-cost candidate generation and batched
verification. The implementation requirements were therefore specified
independently from the MIT baseline and public greedy speculative-decoding
semantics:

- Verification commits the longest candidate prefix that exactly matches the
  target predictions, followed by exactly one target-selected bonus token.
- Output statistics count logical new tokens, not internal replay framing.
- Acceptance histograms retain every cycle when an adaptive block size grows
  beyond its initial value.

Commit `339756698413223894d4114427764773dd4428b3` implements one pure
acceptance planner shared by the legacy
linear and DFlash verification paths and adds exhaustive acceptance-shape,
malformed-batch, logical-accounting, and adaptive-histogram tests. The full
`hipfire-arch-qwen35` library suite passes (28 tests).

On GPU0, the fast 27B DFlash coherence battery passed both prose and code
cells. Both outputs were manually reviewed as coherent and free of token
attractors or special-token corruption. The candidate's three-run fresh-
process median on the fixed 9B DFlash workload was 497.14 tok/s versus 495.86
tok/s for the baseline, with identical emitted-token count and τ=13.1818.
Full identities and measurements are in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

## M2 evidence

### Reproducible delivery

Commit `5dd0415d7f0be08f8322fbaaca67f0886729ae97` makes the Linux
installer and in-place updater source-only clean-room flows. They default to
`HUSRCF/hipfire-mit`, accept an explicit branch, tag, or commit through
`HIPFIRE_INSTALL_REF`, resolve and record the fetched commit, reject dirty or
unpushed updater worktrees, and build with the lockfile. The dynamic Git test
exercises normal updates, both refusal conditions, and detached commit pins.

Commit `0ba62820e50c2b1ebb31e641e3009a44b95a2651` applies the same source
ownership, immutable-ref, provenance, and locked-build contract to Windows,
Nix, and quick-deploy entry points. The static provenance test passes. This
host has neither PowerShell nor Nix, so those two platform scripts have static
validation here and still require native-platform execution before their
support matrix can be called complete.

### Device execution foundation

Commit `793c5b9b3126bf443bafe5ee83d0c1e59ef23dce` removes the silent
`gfx1010` fallback. Hardware discovery must now produce a concrete numeric
target, while an explicit `HIPFIRE_TARGET_ARCH` may select either a concrete
target or a valid LLVM `gfx*-generic` family. Feature suffixes are normalized,
malformed or missing values fail closed, and HIP-version warnings cover the
whole gfx11, gfx115, and gfx12 families.

The same commit closes the public device-ownership invariant: all 380 audited
`Gpu` entry points bind their owning device before doing work. Four pure unit
tests cover target normalization, explicit overrides, fail-closed errors, and
HIP-version families. Both bind-thread verification scripts, the
`rdna-compute` library suite, the clean-room source/license gate, and a GPU0
minimal forward pass pass. The live device was resolved as `gfx1100` with
48.3 GB VRAM and HIP 7.14.

Three GPU0 speed-gate observations produced a median 1310.5 prefill tok/s and
141.3 decode tok/s, respectively +6.78% and +0.28% over the committed W7900
floor. The post-commit short coherence battery passed, and all four generated
outputs were manually reviewed. Full measurements are in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

The agentic gate exposed a deterministic malformed tool-call JSON response on
the installed Qwen 3.6 27B model. A clean detached parent build and the
candidate build had different daemon hashes but produced byte-identical bad
JSON twice on the candidate and once on the parent. This proves it predates
the device change; the targeted agentic skip used for this one commit is
recorded rather than presented as a pass. Tool-call structural correctness is
the next independent repair item.
