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
| M3: quantization and context paths | 19-39 | in progress | strict HFQ boundaries and quant-payload layouts, shared packed-KV allocation, and checked long-context position continuity are complete; quantization numerical fidelity remains open |

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
the installed Qwen 3.6 27B model. A clean detached parent build and the device
candidate build had different daemon hashes but produced byte-identical bad
JSON twice on the candidate and once on the parent, proving it predated the
device change. The targeted agentic skip used for that one commit is recorded
rather than presented as a pass.

Commit `76dd2b97bd83ab61839f7bf02988f41c49f8c2fb` independently closes
that structural failure with a narrow generation constraint. It only activates
inside an unclosed Hermes tool-call body stopped exactly after the first
`"name":` key and only when the proposed value begins as an unquoted
identifier. The valid JSON string prefix is committed through the normal KV
path before resampling; unrelated malformed output remains visible to the
validator. The regular decode path maintains only an O(1) tool-call depth
counter.

The fixed 27B fast cell now passes with valid `name` and `arguments` and zero
warnings. In the full eight-cell matrix, all four 27B cells and both 35B Pi
cells pass; there are zero structural hard failures. Two 35B Hermes cells
initially soft-warned because that model terminated without selecting a tool.
Commit
`658baa8a7fd5e2f45fd4ffadd7577e89391702ba` adds the daemon, tool-call
module, and agentic gate itself to pre-commit hotspot coverage, with a static
test protecting those mappings.

Commit `cab7bb2e2207cc42d70bfbe7db6bb193e6436b5d` closes that remaining
35B Hermes reliability item without changing the gate threshold. If a model
selects a terminator immediately after an otherwise empty unclosed `<think>`
block, the daemon commits `</think>\n` through the normal KV path and resamples
once. The constraint does not alter non-empty or already-closed thinking and
cannot fire more than once per response. Fast mode also accepts an explicit
`HIPFIRE_AGENTIC_FAST_MODEL=3.5|3.6` selector for deterministic diagnosis.

All 166 `hipfire-runtime` library tests pass. The complete GPU0 agentic matrix
now passes all eight cells with zero hard failures and zero soft warnings,
including both Qwen 3.5 35B Hermes cases and both multi-turn cases. Three
GPU0 performance observations for the final agentic batch have median 1288.8
prefill tok/s and 141.6 decode tok/s, respectively +5.01% and +0.50% over the
committed W7900 floor. Detailed quality reports and measurements are recorded
in `docs/cleanroom/PERFORMANCE_RECORD.md`.

## M3 evidence

### HFQ file-boundary contract

Commit `6088d29a2bc932cbb4b38be3ac1bb2d1299e04a5` replaces input-derived
HFQ slicing and assertions with a checked, fallible version-1 parser. It
validates the fixed header, ordered and host-representable offsets, metadata
JSON syntax and object shape, header/index tensor-count agreement, bounded
index records, UTF-8 and unique tensor names, checked cumulative payload
ranges, and complete declared payloads before exposing any tensor slice.
Metadata syntax is validated with a streaming ignored-value deserializer so
the check does not allocate a second tokenizer/config tree during model load.

Six new adversarial unit tests cover a valid multi-tensor file plus truncated
headers, bad magic and versions, invalid offsets and metadata, impossible or
mismatched counts, duplicate/non-UTF-8 names, and truncated index/payload
data. All 172 `hipfire-runtime` library tests pass. Existing Qwen 3.5 MQ4,
Qwen 3.6 MQ3, and MQ4 DFlash files pass CPU index reads; the MQ3 27B model
also completes a GPU0 load, graph capture, short generation, and clean unload.

The mandatory GPU0 short coherence battery and full DFlash/DDTree battery
pass without hard errors or repetition warnings. The commit-hook agentic cell
also passes with valid tool JSON and zero warnings. Three pre-commit speed
observations have median 1387.0 prefill tok/s and 141.1 decode tok/s. The
prefill reading is not attributed to the parser because inference timing
starts after the one-time file parse; it is recorded only as non-regression.
Detailed reports and measurements are in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

### HFQ quant-payload layout contract

Commit `ab1163dcd08ada578e2a9f38aaf71a90396c7b7e` extends the checked HFQ
index boundary from generic byte ranges to the registered quantization
layouts. Before any tensor range can reach a GPU loader, the parser now
requires its quantization type, group size, shape, and exact declared byte
length to agree. Dense F16/F32/BF16 tensors, every registered block format,
the row-aligned Q8HFQ layout, and the HFP4/MFP4 row layouts use checked host
arithmetic; reserved or unknown type identifiers fail closed.

Two new tests cover every active quantization identifier and representative
metadata, shape, group-size, and payload-length failures. Together with the
updated valid fixture, all 183 runtime library tests pass. A CPU-only scan of
20 complete local HFQ files spanning Qwen 3, Qwen 3.5, Qwen 3.6, Laguna,
MQ3/MQ4/MQ4P/MQ4R/HF4, MTP, and DFlash variants also passes, without reading
or scanning tensor payload contents.

GPU0 validation passes the four available standard coherence cells and all
four DFlash/DDTree cells after manual output review; every speculative
detector reports `ok=true` and `soft_warn=false`. The commit-hook Qwen 3.6
27B agentic cell also passes with zero warnings. Three fresh speed-gate runs
have median 1290.6 prefill tok/s and 140.0 decode tok/s, respectively +5.16%
and -0.64% versus the committed W7900 floor. This is recorded as
non-regression because the new work occurs once during model-index parsing,
outside the measured inference hot path. Detailed identities and reports are
in `docs/cleanroom/PERFORMANCE_RECORD.md`.

### Packed KV allocation contract

Direction rows 20-27 expose the high-level requirement that context state
remain capacity-efficient without losing numerical continuity. An audit of
the permitted MIT snapshot found the Q8 and rotated 2/3/4-bit cache byte
layouts repeated across single-GPU, filtered-layer, capped, and multi-GPU
constructors. The repeated arithmetic used assertions and unchecked host-size
multiplication, so equivalent modes could drift in allocation size and an
invalid configuration could panic or overflow before allocation.

Commit `c3dfe24fd2734cfef21f30b2cac6e7f3daab5d8d` introduces one pure
host-side packed-layout contract without changing the cache payload format,
rotation parameters, kernels, or dispatch. It validates positive dimensions,
32-element Q8 block alignment, the established 128/256-dimensional 2/4-bit
and 256-dimensional 3-bit shapes, physical capacity within the logical
context, and every multiplication and F32-storage rounding step. All matching
single-GPU, filtered, capped, and multi-GPU constructors now consume the same
computed K/V element counts and per-head byte strides and return a fallible
HIP error for invalid inputs instead of asserting.

Five deterministic tests pin exact Q8 and asymmetric byte counts, rounding,
physical-cap scaling, invalid shape/capacity rejection, and host address-space
overflow. All 177 `hipfire-runtime` library tests pass. GPU0 short coherence
and the four-cell DFlash/DDTree battery pass; every available output was
manually checked and all speculative detectors report `ok=true` and
`soft_warn=false`. The commit-hook agentic cell also passes with valid tool
JSON and no warning.

Three measurements from the explicitly rebuilt candidate benchmark have a
median 1288.1 prefill tok/s and 140.6 decode tok/s, respectively +4.95% and
-0.21% versus the committed W7900 floor. This is recorded as non-regression,
not as a speedup caused by host-side allocation validation. During evidence
collection, the speed gate was found to accept an existing but stale linked
benchmark; commit
`217645bef42d1ca79e3bf4ab4a35c61b96785e20` now always asks Cargo to validate
freshness and only relinks stale targets before taking measurements. Detailed
binary hashes, reports, and observations are in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

### Long-context eviction position contract

Direction rows 20-27 also require repeated cache compaction to preserve the
logical token position while physical storage is recycled. The permitted MIT
snapshot maintained that position with repeated unchecked additions in the
plain TriAttention and CASK paths and asserted the eviction schedule during
construction. That left host-size overflow and mismatched physical-capacity
inputs outside the fallible runtime contract.

Commit `4c2733d99dcf7fda94f22d4b449ba06837c50f07` introduces one checked
position plan shared by both policies. It validates the positive retention
budget, the overflow-safe `budget + beta` trigger (including the documented
`beta = 0` configuration), current physical capacity, absolute-position
addition, and compact-offset addition before dispatch. A successful plan
enforces `new_physical + new_compact_offset = old_physical + old_offset`, and
cache state is updated only after every layer compacts successfully.

Four deterministic tests cover schedule capacity and overflow, below-trigger
behavior, two successive evictions, current-capacity rejection, and offset
overflow. All 181 runtime library tests and all runtime examples with the
DeltaNet feature pass; the clean-room source/license gate passes.

On GPU0, a 9B asym3 tight-cache run used 42 physical slots for a 122-token
prompt plus 32 generated tokens and completed 15 TriAttention evictions. A
separate Q8 CASK run completed five m-fold evictions and ended with
`compact_offset = 40`. The standard four-cell coherence battery and the
four-cell DFlash/DDTree battery pass after manual output review; every
speculative detector reports `ok=true` and `soft_warn=false`.

Three freshly linked speed-gate observations have median 1291.1 prefill tok/s
and 140.4 decode tok/s, respectively +5.20% and -0.35% versus the committed
W7900 floor. This is recorded only as non-regression because the new checks
execute on the opt-in eviction path, not the measured default benchmark path.
Full run identities, binary hashes, and reports are in
`docs/cleanroom/PERFORMANCE_RECORD.md`.
