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
| M3: quantization and context paths | 19-43 | in progress | strict HFQ boundaries and quant-payload layouts, ten-cell GPU/CPU quant parity gating including MQ8/MQ4, a reproducible MQ8 scalar-dot negative experiment, shared packed-KV allocation, and checked long-context position continuity are complete; full model-level format fidelity remains open |
| M4: kernel fault localization | 44 | complete | deterministic kernel artifact provenance and staged compile/load/symbol failure context |
| M5: clean-room composition | 45-46 | complete | one reproducible CPU/GPU integration gate, evidence manifest, fail-closed GPU selection, and mandatory generated-output review |
| M6: multimodal input boundary | 47 | complete | one checked image-to-patch representation shared by all VL frontends, with exact visual-token metadata and fallible malformed-input handling |
| M7: unified generation stops | 48 | complete | one stop-token union shared by all user-facing generation paths, with static drift detection |
| M8: architecture-family adapters | 49 | complete | adapter-owned model-family identifiers, fail-closed unknown families, and shared execution foundations |
| M9: HFQ consumer shapes | 50 | complete | load-time binding between file-declared payload shapes and model-consumer dimensions |
| M10: tokenizer special-token scan | 51 | complete | byte-identical single-pass special-token discovery with reproducible CPU and GPU0 performance evidence |
| M11: speculative seed-repeat embedding | 52 | complete | exact Q8 seed-plus-mask block embedding in one GPU launch, shared by all DFlash/DDTree draft paths |
| M12: packed KV footprint record | 53 | complete | deterministic four-format capacity records derived from the checked allocation contract |
| M13: generic batched target verify | 54 | complete | one shared batched target verification call with reusable scratch and greedy-state parity |
| M14: hybrid-aware daemon KV allocation | 55 | complete | production Q8/default-Asym3 caches allocate full storage only for FullAttention layers |

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

### Quantization numerical parity gate

Commit `dce4648a97490426b13ed267a81b27ef327c5811` turns five existing
synthetic CPU-reference/GPU comparisons into one mandatory, report-producing
gate. It covers HFQ4-G256 GEMV, embedding, and MMQ residual output; HFQ6-G256
GEMV; MQ3-G256-Lloyd tail groups; HFP4-G32 tail groups; and rotated MFP4-G32
tail groups. Every case owns a numerical tolerance and a non-zero failure
exit, the script builds current release targets before execution, and all GPU
work is serialized through `scripts/gpu-lock.sh`.

The pre-commit hook now runs this battery when staged paths indicate
quantization, dequantization, HFQ/MQ, HFP4, or MFP4 work. On GPU0 all five
cases pass; the report records HFQ4 GEMV error `0.000366`, HFQ4 MMQ residual
error `0.040617`, HFQ6 error `0.000061`, and passing quad/tail sweeps for the
remaining formats. Standard coherence and the 27B agentic cell also pass
after manual review.

Three fresh performance observations have median 1302.9 prefill tok/s and
140.4 decode tok/s, respectively +6.16% and -0.35% versus the committed W7900
floor. This testing-only change does not alter inference binaries, so the
measurements establish non-regression only. Full report identities are in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

### Expanded registered-format parity coverage

Commit `912d193ade56fb41db940f01036b7f7a680c9329` expands the mandatory
quantization battery from five representative cells to nine. A new synthetic,
model-independent anchor constructs valid Q4K blocks with packed high
scale/min bits, derives Q4F16-G32 and G64 payloads, constructs Q8_0 and
row-aligned Q8HFQ payloads, and compares every GPU GEMV against an independent
CPU dequantization reference. It removes the evidence gap left by historical
Q4 tests that skip when a developer-specific TinyLlama GGUF is absent.

The gate now also includes rotated MQ3/MQ2 GEMV with isolated FWHT parity,
HFQ3 residual GEMV across four shapes, and Q8 KV write/attention, in addition
to the prior HFQ4, HFQ6, MQ3-Lloyd, HFP4, and MFP4 cases. All nine cells pass
on GPU0. Classic-format maximum absolute errors are `2.38e-7` for Q4K,
`4.81e-4` for Q4F16-G32, `4.76e-4` for Q4F16-G64, `9.50e-8` for Q8_0,
and `6.33e-8` for Q8HFQ. MQ rotation is element-exact, all HFQ3 residual
shapes are exact, and the Q8 KV round-trip remains below its fixed threshold.

All 183 runtime library tests and every runtime example with DeltaNet pass.
Standard coherence and all four DFlash/DDTree cells pass after manual review.
Three fresh speed runs have median 1324.8 prefill tok/s and 141.2 decode
tok/s, respectively +7.94% and +0.21% versus the committed W7900 floor. The
inference binaries are unchanged, so these are non-regression observations,
not an attributed speedup. Detailed evidence is in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

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

### MQ6 common-loader closure

Commit `a432e8694356e4c1993266f75d8096d051e1c9d2` closes a format-routing gap
found while expanding the quantization coverage matrix. The MIT quantizer can
emit `quant_type=15` MQ6-G256 tensors, the HFQ index validator accepts their
200-byte/256-element layout, and the runtime already dispatches MQ6 kernels,
but the common LLaMA-family HFQ weight loader omitted the corresponding dtype
arm. Such a model therefore failed during loading before reaching the existing
kernel. The loader now uploads the opaque payload as `DType::MQ6G256`, matching
the already-supported Qwen3.5 route.

The rotated MQ parity anchor now independently transforms and quantizes
synthetic weights to MQ6, decodes its packed six-bit payload on the CPU, and
compares that reference with the GPU rotate-plus-GEMV path. GPU0 maximum
absolute errors are `6.10e-5`, `2.44e-4`, and `8.54e-4` at K=256, 512, and
1024, all below the fixed `1e-3` limit; the standalone FWHT remains
element-exact. The unified nine-cell quantization gate, 183 runtime library
tests, all DeltaNet runtime examples, and the clean-room license gate pass.

The four available standard coherence cells and all four DFlash/DDTree cells
pass after manual review; speculative detectors report `ok=true` and
`soft_warn=false`. The commit-hook Qwen3.6 27B agentic cell emits valid `read`
JSON with zero warnings. Three fresh speed observations have median 1269.6
prefill tok/s and 140.3 decode tok/s, respectively +3.45% and -0.43% versus
the committed W7900 floor; the hook repetition also passes. This records
non-regression only, because the change activates an existing MQ6 execution
path rather than altering its kernel. Detailed evidence is in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

### Compact HFQ and MQ2-Lloyd parity closure

Commit `73be35f32d3ab833b2bc058636fffa0b7374cd3c` expands the mandatory
quantization gate from nine to ten cells and closes five remaining registered
GEMV evidence gaps without requiring model files. The new synthetic anchor
constructs and independently decodes HFQ4-G128, HFQ2-G256, HFQ2-G128, and
HFQ3-G128 payloads, plus an MQ2-G256-Lloyd codebook payload with CPU FWHT
rotation. It sweeps K=256, 512, and 1024 for every format.

All 15 new GPU0 comparisons pass. The largest absolute error is
`1.464844e-3` for HFQ3-G128 at K=1024, below the fixed `2e-3` threshold;
MQ2-Lloyd remains at or below `3.814697e-5`. The complete ten-cell parity
battery, all 183 runtime library tests, every DeltaNet runtime example, and
the clean-room license gate pass. Standard coherence and all four
DFlash/DDTree cells pass after manual review; every speculative detector is
`ok=true` with `soft_warn=false`. The Qwen3.6 27B agentic hook cell also
passes with valid tool JSON and no warnings.

Three fresh performance observations have median 1304.4 prefill tok/s and
139.9 decode tok/s, respectively +6.28% and -0.71% versus the committed
W7900 floor; the hook repetition also passes. The inference executable hashes
are unchanged because this batch adds only a numerical test, so the values
establish non-regression and no speedup is attributed. Detailed evidence is
in `docs/cleanroom/PERFORMANCE_RECORD.md`.

### MQ8 gfx11 execution and MQ4/MQ8 parity closure

Commit `a36f989635a531dbff5a5e61e7f3e0e416c7de0c` closes the final
registered integer-MQ GEMV gaps in the synthetic parity anchor. The audit
found that the MQ8 kernel called the signed `sdot4` builtin directly, which
the local ROCm 7.14 compiler rejects for `gfx1100` because that target does
not expose the older `dot1-insts` feature. The kernel now uses gfx11/gfx12's
generalized six-argument `sudot4` with both operand-sign flags set, while
gfx9/gfx10 retain their existing signed `sdot4` path.

The MQ anchor now independently constructs MQ8-G256 and MQ4-G256 weights,
rotates and quantizes activations for MQ8, decodes both payloads on the CPU,
and compares them with GPU rotate-plus-GEMV at K=256, 512, and 1024. On GPU0,
MQ8's maximum absolute error is `1.220703e-4` under its `1e-3` budget. MQ4's
maximum is `1.220703e-3` under a format-specific `2e-3` budget that accounts
for its 32-lane FP32 tree reduction versus the independent serial CPU sum.
The isolated FWHT remains element-exact at every shape. The complete
ten-cell quantization battery passes.

The full workspace all-target test suite, every workspace example check, and
the clean-room source/license gate pass. Four available standard coherence
cells and all four DFlash/DDTree cells pass after manual review; speculative
detectors are `ok=true` with `soft_warn=false`. The commit-hook agentic cell
also emits valid `read` JSON with no warnings.

Three fresh GPU0 performance observations have median 1345.1 prefill tok/s
and 140.0 decode tok/s, respectively +9.60% and -0.64% versus the committed
W7900 floor. The hook repetition passes at 1320.3 and 140.1 tok/s. The MQ4
benchmark establishes default-path non-regression; no speedup is attributed
to the MQ8-only execution repair. Detailed evidence is in
`docs/cleanroom/PERFORMANCE_RECORD.md`.

### MQ8 scalar-dot negative experiment

Commit `fcbc0489ac923e6bbc99f56ef4515a01a825548a` adds a reproducible
GPU microbenchmark for direction row 43 without changing the production
kernel. It compiles the current gfx11 MQ8 GEMV twice: the control keeps the
signed hardware `sudot4`, while the candidate expands every packed dot into
four explicit scalar integer multiplies. Both variants use identical
synthetic MQ8 payloads and activations and must agree numerically before any
timing is accepted.

Across four projection shapes and three fresh GPU0 processes after explicit
five-second DPM warm-up, every output is bit-exact. The scalar/control latency
ratio has a process median of 1.639; per-shape medians range from 1.242 to
1.791. HSACO inspection confirms ten `v_dot4_i32_iu8` instructions and 33
VGPRs for the control versus forty `v_mul_lo_u32` instructions and 51 VGPRs
for the scalar candidate, with no scratch or spills in either variant. The
experiment therefore rejects the scalar fallback and retains the existing
gfx11 signed hardware-dot implementation.

The ten-cell quantization battery, full workspace all-target tests, workspace
example check, clean-room gate, four available standard coherence cells, all
four DFlash/DDTree cells, and the fast agentic cell pass. Three fresh default
MQ4 speed-gate observations have medians 1371.5 prefill tok/s and 139.7 decode
tok/s, respectively +11.75% and -0.85% versus the committed W7900 floor. This
is non-regression evidence only: the committed file is an opt-in benchmark
and does not modify inference execution. Full identities, hashes, ISA
evidence, and observations are in `docs/cleanroom/PERFORMANCE_RECORD.md`.

## M4 evidence

### Kernel artifact provenance and staged failures

Commit `19a8e6d6d14bff8dec6af4231c936eb9da53f92b` adds host-side
observability at the JIT boundary without changing a kernel or the launch hot
path. Each selected module now retains its module name, target architecture,
combined source-and-architecture hash, artifact path, validation state, and
one of four origins: validated precompiled, unvalidated packaged fallback,
validated cache, or runtime compilation. The public query returns a cloned,
module-sorted snapshot; it does no device work.

Kernel initialization failures now preserve the original HIP error code and
identify whether they occurred during source compilation, HSACO module load,
or function-symbol lookup. Every error also includes module, function,
architecture, and the artifact path when one exists. This converts formerly
ambiguous loader failures into a layer-specific diagnostic without exposing
kernel source text.

Eight deterministic `rdna-compute` tests cover all four origins, validation
semantics, architecture-sensitive source identity, deterministic ordering,
and all three failure stages. The final GPU0 example reports `gfx1100`,
`source_arch_hash=6e0b453068533574`, `origin=ValidatedCache`, and the exact
validated HSACO path. An earlier fresh-cache invocation exercised
`RuntimeCompiled` with the same identity.

The full workspace all-target suite, workspace example check, clean-room
gate, ten-cell quantization battery, four available standard coherence cells,
all four DFlash/DDTree cells, and the Qwen3.6 agentic cell pass. Five fresh
GPU0 speed observations have medians 1252.1 prefill tok/s and 140.3 decode
tok/s, respectively +2.02% and -0.43% versus the W7900 floor; their prefill
relative spread is 2.27%. The commit-hook repetition passes at 1369.9 and
140.8 tok/s. No speedup is attributed to initialization-only diagnostics.
Full evidence is in `docs/cleanroom/PERFORMANCE_RECORD.md`.

## M5 evidence

### Reproducible integration gate

Direction rows 45-46 require independently developed work to be composed
under regression control. The existing pre-commit hook deliberately selects
checks by changed path, so it did not provide a single explicit proof that all
currently active clean-room paths still compose.

Commit `0567fb08e6800be837be786702e8ad3c1d9d028d` adds
`scripts/cleanroom-integration-gate.sh` and a deterministic plan/self-check
test. CPU mode combines the source diff check, full locked workspace all-target
tests, locked workspace example checking, public-device binding audit,
agentic-detector self-check, and MIT source/license gate. Full mode then runs
the ten-cell quantization parity battery, standard and DFlash/DDTree coherence
batteries, the fast agentic structure gate, and three to five fresh speed-gate
processes. Every delegated GPU gate keeps its existing GPU lock.

Full mode fails closed unless both ROCr and HIP visibility select the requested
device; the accepted run selected device 0 for both. Every step has a separate
log and the generated summary pins the candidate parent, branch, dirty-candidate
diff hash, device visibility, ordered statuses, and report paths. A machine
pass does not waive manual review of generated text.

The complete GPU0 run passes all CPU steps, all ten quantization cases, four
available standard coherence cells, all four DFlash/DDTree cells, and the
Qwen3.6 27B agentic cell with zero hard failures and zero soft warnings.
Generated outputs were manually reviewed: the two standard short-mode samples
that end mid-answer are bounded-generation truncations, while all available
text remains on-topic and free of loops or special-token corruption. The
DFlash/DDTree prose and code are coherent and every detector reports
`ok=true` and `soft_warn=false`; the agentic call is valid JSON and satisfies
its schema.

Three fresh GPU0 speed observations have medians 1282.1 prefill tok/s and
140.8 decode tok/s, respectively +4.47% and -0.07% versus the committed W7900
floor. Relative to M4's five-run medians they differ by +2.40% and +0.36%, so
the cross-batch 5% expansion rule did not require two additional runs. This is
composition non-regression evidence; the batch changes scripts only and no
performance gain is attributed.

PFlash is explicitly reported as skipped because the required local target and
drafter models are absent; it is not counted as passing coverage. Runtime
multi-GPU execution is also outside this GPU0-only acceptance run, while the
existing multi-GPU device-binding static audit remains included. Full evidence
is in `docs/cleanroom/PERFORMANCE_RECORD.md` and the run manifest at
`/tmp/hipfire-integration-row45-46/summary.md`.

## M6 evidence

### Checked visual-input normalization

Direction row 47 requires non-text input to be normalized into a
model-consumable representation while keeping interfaces consistent. The
permitted snapshot decoded and normalized an image, derived its grid and token
count, and extracted patches as separate operations repeated by `infer`,
`infer_vl`, and the daemon. The daemon's early context estimate assumed a
fixed 448-by-448 image even though smart resize can produce a different shape,
so admission accounting could disagree with the representation later sent to
the model.

Commit `c0ba0b7930da81957bff5980f4d66bbc7162eefe` introduces an immutable
`PreparedImage` boundary. Filesystem paths and already-decoded images share
the same normalization implementation and produce checked patch data,
resized dimensions, patch-grid dimensions, and the exact merged visual-token
count. The three frontends consume those getters directly; callers no longer
repeat geometry arithmetic. The daemon now admits requests using the true
smart-resize token count and returns a structured error for an invalid image
instead of panicking during decode.

The preprocessing boundary validates positive patch, temporal, and merger
geometry; checked resize-factor, area, CHW, patch, and token-count arithmetic;
the image decoder's dimension limit; a 200:1 aspect-ratio safety limit; exact
CHW length; patch divisibility; and merger-grid divisibility. Four new tests
cover path/in-memory representation identity, derived-shape invariants,
invalid model geometry, malformed CHW input, partial patches, zero dimensions,
extreme aspect ratios, and resize alignment. The existing four pure-color
channel-order tests continue to pass.

The full locked workspace all-target suite, all workspace examples, binding
audits, MIT gate, ten-cell quantization parity battery, four available
standard coherence cells, all four DFlash/DDTree cells, and the Qwen3.6 27B
agentic cell pass. Generated outputs were manually reviewed and the
speculative/agentic detectors report no warning. Three fresh GPU0 performance
observations have medians 1275.8 prefill tok/s and 141.0 decode tok/s,
respectively +3.95% and +0.07% versus the W7900 floor. Relative to M5 they are
-0.49% and +0.14%, so the 5% cross-batch expansion rule did not fire. The
commit-hook repetition passes at 1289.3 and 140.9 tok/s.

No installed model contains a vision configuration, so this host cannot run
an end-to-end vision-tower GPU inference. That environment gap is not counted
as a pass: the accepted evidence covers deterministic preprocessing parity,
all VL entry-point compilation, and default-path GPU0 non-regression. Full
identities and reports are in `docs/cleanroom/PERFORMANCE_RECORD.md`.

The conservative direction-table position is now complete through row 47 of
2706; row 48 is next. Direction rows are audit inputs and are aggregated into
independently specified implementation batches, so the remaining 2659 rows
are not a promise of 2659 one-to-one Git commits.

## M7 evidence

### Unified generation-stop semantics

Direction row 48 requires prompt encoding, templates, sampling, and stopping
to preserve the model's input/output semantics. The permitted snapshot already
centralized prompt framing and sampling policy, but its decode loops had
drifted: the tokenizer defined both its primary EOS and an auxiliary EOT as
terminators, while several user-facing entry points compared only model EOS
and, in some cases, a ChatML frame stop. A raw-text generation that emitted
the auxiliary EOT could therefore continue into a post-terminator attractor
loop depending on the selected frontend.

Commit `9c794f3743d2fb78b1f127e54dcff8bcb57b9b5d` adds one pure
`is_generation_stop` contract that takes the union of model metadata,
tokenizer EOS/EOT metadata, and the active frame stop. The daemon, interactive
runner, text and vision inference paths, and the Qwen3, Qwen3.5, and HFQ
frontends all use that contract. Two unit tests cover every stop source,
unrelated tokens, absent frame stops, and duplicate identifiers. A new static
audit covers all seven user-facing entry points and is now part of every
clean-room integration run, preventing a future bare-EOS regression.

The full locked workspace all-target suite, all workspace examples, binding
audits, MIT gate, ten-cell quantization parity battery, four available
standard coherence cells, all four DFlash/DDTree cells, and the Qwen3.6 27B
agentic cell pass on GPU0. Manual review found coherent bounded outputs, no
special-token corruption or attractor loop, `ok=true`/`soft_warn=false` for
every speculative cell, and valid tool-call JSON with zero warnings.

Three fresh performance processes measure 1300.3, 1302.2, and 1282.5 prefill
tok/s and 140.9, 140.5, and 140.3 decode tok/s. Their medians are 1300.3 and
140.5 tok/s, respectively +5.95% and -0.28% versus the committed W7900 floor.
Relative to M6 the median changes are +1.92% and -0.35%, so the 5% cross-batch
expansion rule did not fire. The commit-hook repetition measured 1313.8 and
141.5 tok/s. This change replaces existing stop comparisons with an inlined
constant-time union and claims semantic consistency, not a speedup.

The conservative direction-table position is now complete through row 48 of
2706; row 49 is next. The remaining direction count is 2658. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M8 evidence

### Adapter-owned architecture families

Direction row 49 requires architecture differences to remain behind dedicated
adapters while execution infrastructure stays shared. The permitted snapshot
already exposed an `Architecture` adapter trait, but the daemon still selected
families with hard-coded numeric tests and silently treated every unknown
architecture identifier as LLaMA. That fallback could bind an unsupported
model to the wrong implementation instead of rejecting it.

Commit `22848b873e4fe14264d568241a11fce4d0c36404` adds an adapter-owned
`supports_arch_id` contract. LLaMA owns identifiers 0 and 1, Qwen3.5 and its
VL adapter own identifiers 5 and 6, and Toy retains its single canonical
identifier. Daemon model loading, family labels, pipeline-parallel checks,
and Qwen-family capability checks now query those adapters. Unknown identifiers
fail closed with an explicit error. Protocol-visible family labels are unchanged,
and execution hot paths remain statically dispatched; the new family resolution
is confined to cold model-selection paths.

Per-adapter unit tests cover canonical and alternate identifiers, daemon tests
cover all supported families, fail-closed behavior, and stable protocol labels,
and a new architecture-adapter audit is part of every clean-room integration
run. The full locked workspace all-target suite, all workspace examples,
binding audits, MIT gate, ten-cell quantization parity battery, four available
standard coherence cells, all four DFlash/DDTree cells, and the Qwen3.6 27B
agentic cell pass on GPU0. Manual review found coherent bounded outputs, valid
tool-call JSON, and no speculative or agentic warning.

The first three fresh performance processes had an elevated prefill median, so
the cross-batch 5% rule expanded the sample to five independent processes. The
five observations are 1314.2, 1376.2, 1378.3, 1226.7, and 1314.5 prefill tok/s,
and 141.4, 141.4, 140.5, 140.7, and 140.2 decode tok/s. Their medians are 1314.5
and 140.7 tok/s. Relative to M7 they change by +1.09% and +0.14%; relative to
the committed W7900 floor they change by +7.11% and -0.14%. The commit-hook
repetition passes at 1251.4 and 141.4 tok/s. This batch claims adapter-boundary
correctness and no regression, not a speedup.

The conservative direction-table position is now complete through row 49 of
2706; row 50 is next. The remaining direction count is 2657. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M9 evidence

### HFQ consumer-shape contract

Direction row 50 requires quantized formats to make compatibility, numerical,
storage, and throughput tradeoffs explicit. The permitted implementation
already validated that every HFQ payload matched the shape declared by its own
index entry. Model loaders, however, could then interpret those bytes using
dimensions derived from model configuration without proving that the two
shapes agreed. A file could therefore be internally self-consistent yet be
handed to a GPU kernel with incompatible matrix dimensions.

Commit `dc2a8ed0aa1ab80cb55884b13823ae38e9e2e4a6` adds cold-path
`expect_shape` and `expect_numel` contracts to HFQ tensor metadata. LLaMA,
Qwen3.5, and DFlash matrix loaders now bind file-declared shapes to the model's
expected dimensions before upload. Embedding and normalization loads are also
checked, while the Qwen raw-byte loader now requires complete tensor metadata
instead of accepting a detached numeric quantization identifier. Explicitly
flattened DeltaNet tensors use the element-count contract. No validation is
placed in inference or GPU dispatch hot paths.

A negative unit test proves exact-shape and flattened-count acceptance and
rejects incompatible consumer dimensions. A static audit prevents the Qwen raw
loader from reverting to quant-type-only calls and requires matrix checks in
all three covered loaders; the audit is part of every integration run. The
full locked workspace all-target suite, all workspace examples, binding and
clean-room audits, ten-cell quantization parity battery, four available
standard coherence cells, all four DFlash/DDTree cells, and the Qwen3.6 27B
agentic cell pass on GPU0. Manual review found coherent bounded text, valid
tool-call JSON, and no speculative or agentic warning.

Three fresh performance processes measure 1297.1, 1249.7, and 1278.2 prefill
tok/s and 141.3, 140.8, and 140.3 decode tok/s. Their medians are 1278.2 and
140.8 tok/s. Relative to M8 they change by -2.76% and +0.07%, so the 5%
cross-batch expansion rule did not fire. Relative to the committed W7900 floor
they change by +4.15% and -0.07%. The commit-hook repetition passes at 1303.4
and 141.1 tok/s. This batch claims safer format compatibility and no runtime
regression, not a speedup.

The conservative direction-table position is now complete through row 50 of
2706; row 51 is next. The remaining direction count is 2656. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M10 evidence

### Single-pass special-token discovery

Direction row 51 again emphasizes tokenizer and generation-path performance
without relaxing input/output semantics. An independent audit of the permitted
implementation found that every encode call searched the remaining prompt once
for each registered special token. Ordinary user text normally contains none,
so its common path repeatedly scanned the same bytes before BPE work began.

Commit `df8c5408cbfd773513e668d9c974f47980a6e8a8` builds a fixed 256-bucket
index from the first UTF-8 byte of each special token. Encoding now scans the
input once and compares only the longest-first candidates that can begin at the
current byte. The bucket order preserves the prior earliest-position and
longest-at-position rules. UTF-8 continuation bytes never populate a bucket,
and full-string `starts_with` checks still decide every match.

A byte-token synthetic tokenizer compares the indexed implementation against
the retained linear reference for ordinary text, overlapping and adjacent
specials, non-angle delimiters, incomplete markers, and multibyte Unicode.
All 187 runtime library tests pass. A static audit requires the fixed index,
single-pass scanner, reference-parity test, and reproducible microbenchmark,
and rejects reintroduction of exhaustive production searches; it is included
in every integration run.

On the same 4B MQ4 tokenizer and 3,000 iterations per sample, three fresh
processes reduce plain-prompt median encode latency from 23,296.3 ns to
22,496.5 ns (-3.43%) and framed-prompt latency from 12,955.7 ns to 10,880.2 ns
(-16.02%). The full locked workspace all-target suite, all examples, clean-room
audits, ten-cell quantization parity battery, four available standard coherence
cells, four DFlash/DDTree cells, and the Qwen3.6 27B agentic cell also pass on
GPU0. Manual review found coherent text, valid tool-call JSON, and no
speculative or agentic warning.

Three fresh GPU0 performance processes measure 1406.5, 1298.9, and 1276.0
prefill tok/s and 141.7, 141.6, and 140.6 decode tok/s. Their medians are
1298.9 and 141.6 tok/s. Relative to M9 they change by +1.62% and +0.57%, so the
5% cross-batch expansion rule did not fire. Relative to the committed W7900
floor they change by +5.84% and +0.50%. The GPU measurements establish full
inference non-regression; the attributed speedup is limited to the separately
measured CPU tokenizer path.

The conservative direction-table position is now complete through row 51 of
2706; row 52 is next. The remaining direction count is 2655. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M11 evidence

### One-launch speculative input embedding

Direction row 52 calls for low-cost candidate generation and batched
verification. An independent audit of the permitted DFlash implementation
found that every draft cycle constructs the fixed input shape `[seed, mask,
mask, ...]`, but the installed 27B Q8 target launched one embedding kernel per
row. The repeated mask rows therefore paid serial launch overhead even though
their token identifier and lookup table row are identical.

Commit `d283ab580696390018b0e67be0b06ab747ad651c` adds a Q8 seed-repeat
embedding kernel that selects the seed row for block zero and the repeated
mask row for every other block. All rows are emitted in one launch, with the
two token identifiers passed as scalar arguments; no per-cycle token-id upload
is introduced. One shared helper applies this path to vanilla DFlash and both
DDTree draft entry points. HFQ4 and F32 formats retain their prior exact
per-row fallback.

A deterministic synthetic Q8 table proves bit-exact equality between seven
single-row reference lookups and the new seven-row launch. The parity case is
the eleventh member of the mandatory GPU battery. A static audit requires the
kernel, dispatch contract, all three call sites, parity anchor, and retained
phase-performance evidence; it is included in every integration run.

Three fresh GPU0 processes measured both the parent and candidate with phase
diagnostics enabled. In the deterministic code cell, the median draft phase
falls from 9,699.2 us to 9,649.8 us (-0.51%), while end-to-end throughput is
149.42 versus 149.30 tok/s (-0.08%). In the stochastic prose cell the median
draft phase is effectively unchanged at 8,469.5 versus 8,469.2 us, and all
three candidate processes remain free of token-attractor warnings. The
optimization therefore removes fifteen serial launches at `B=16` without a
measurable end-to-end regression; the claimed speedup is limited to the
instrumented code-cell draft phase.

The full locked workspace all-target suite, all examples, clean-room audits,
eleven-cell quantization parity battery, four available standard coherence
cells, four DFlash/DDTree cells, and the Qwen3.6 27B agentic cell pass on GPU0.
Manual review found coherent text, valid tool-call JSON, and no speculative or
agentic warning.

Three fresh general GPU0 performance processes measure 1,263.7, 1,359.0, and
1,277.5 prefill tok/s and 140.6, 139.9, and 139.6 decode tok/s. Their medians
are 1,277.5 and 139.9 tok/s. Relative to M10 they change by -1.65% and -1.20%,
so the 5% cross-batch expansion rule did not fire. Relative to the committed
W7900 floor they change by +4.09% and -0.71%.

The conservative direction-table position is now complete through row 52 of
2706; row 53 is next. The remaining direction count is 2654. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M12 evidence

### Allocation-derived context footprint records

Direction row 53 is a record-oriented attention/cache entry. The permitted
implementation already used one overflow-checked packed-layout contract for
single-GPU, filtered, and multi-GPU cache constructors, but capacity evidence
still depended on copied formulas and incidental constructor logs. That made
it possible for a report to drift from the storage actually allocated or from
the byte strides consumed by kernels.

Commit `d0b76b3e815bc0fb0bd3d6b0347a542f3c1953af` exposes a read-only
`packed_kv_footprint` planner. It calls the same `PackedKvLayout` used by the
real constructors and records format, KV-layer count, head shape, logical
context, physical capacity, K/V bytes per head, F32-storage-rounded bytes per
layer, and total K+V allocation. All aggregate arithmetic is checked for host
address-space overflow, and zero KV-layer records fail closed. The planner
does not allocate a GPU and is absent from inference hot paths.

A deterministic example emits JSON for Q8, Asym2, Asym3, and Asym4 at 16 KV
layers, four 256-dimensional KV heads, logical context 65,536, and physical
capacity 2,048. The resulting totals are respectively 68.0, 42.5, 46.5, and
50.5 MiB. Unit tests pin all four totals, the exact Asym3 per-head/per-layer
layout, logical-versus-physical capacity, empty-layer rejection, and aggregate
overflow. A static audit requires the shared layout call, public record, four
formats, and deterministic example; it is part of every integration run.

The full locked workspace all-target suite, all examples, clean-room audits,
eleven-cell quantization parity battery, four available standard coherence
cells, four DFlash/DDTree cells, and the Qwen3.6 27B agentic cell pass on GPU0.
Manual review found on-topic bounded text, valid tool-call JSON, and no
speculative or agentic warning. The 4B code and 9B reasoning cells reach their
short-mode token bounds; their visible analysis remains correct and free of
loops.

Three fresh GPU0 performance processes measure 1,319.6, 1,296.8, and 1,287.6
prefill tok/s and 141.5, 141.1, and 140.6 decode tok/s. Their medians are
1,296.8 and 141.1 tok/s. Relative to M11 they change by +1.51% and +0.86%, so
the 5% cross-batch expansion rule did not fire. Relative to the committed
W7900 floor they change by +5.66% and +0.14%.

The conservative direction-table position is now complete through row 53 of
2706; row 54 is next. The remaining direction count is 2653. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M13 evidence

### Generic two-model batched verification

Direction row 54 calls for low-cost candidate generation and batched target
verification. The DFlash paths already used the shared batch verifier, but the
generic target-plus-draft `spec_step_greedy` entry point still invoked the
target once per candidate and downloaded one full vocabulary row after every
call.

Commit `e25078d4105867823a5c4419632cc182c5826924` routes the complete candidate
block through one `verify_dflash_block` call. The target prediction that
precedes the block is captured first; the batch returns the remaining greedy
predictions, including the full-acceptance bonus. The existing pure acceptance
planner and rollback/commit replay remain unchanged, so the committed token
sequence and final target/draft state contract are preserved. The interactive
runner allocates `VerifyScratch` once per session and supplies a zero-extract
hidden ring, avoiding hidden-history allocation while satisfying the shared
batch interface.

A static audit requires the batch call, returned argmax vector, reusable
scratch at the caller, and absence of a serial `target.forward` in the
verification section. Its test also runs all greedy acceptance shapes and
compiles the wired example. The full locked workspace all-target suite,
workspace examples, clean-room audits, eleven-cell quant parity battery, four
available standard coherence cells, four DFlash/DDTree cells, and the Qwen3.6
27B agentic cell pass on GPU0. Manual review found coherent bounded output,
valid tool-call JSON, and no speculative or agentic warning; the 9B standard
reasoning cell ends at its short-mode bound while its visible reasoning is
correct and non-repetitive.

Three fresh GPU0 performance processes measure 1,295.9, 1,295.6, and 1,292.1
prefill tok/s and 141.4, 140.7, and 140.3 decode tok/s. Their medians are
1,295.6 and 140.7 tok/s. Relative to M12 they change by -0.09% and -0.28%, so
the 5% cross-batch expansion rule did not fire. Relative to the committed
W7900 floor they change by +5.57% and -0.14%.

The conservative direction-table position is now complete through row 54 of
2706; row 55 is next. The remaining direction count is 2652. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M14 evidence

### FullAttention-only production KV allocation

Direction row 55 extends the context path to reduce state capacity and
movement while preserving numerical continuity. Qwen3.5 hybrid models keep
recurrent state for LinearAttention layers and use the packed KV cache only in
FullAttention layers. Although checked capped+filtered Q8 and Asym3
constructors already existed, the single-GPU production daemon still invoked
their unfiltered counterparts and allocated full K/V buffers for every layer.

Commit `0098b82` derives an absolute-layer boolean mask from the loaded model
configuration and routes the reference Q8 and default Asym3 modes through the
capped+filtered constructors. FullAttention buffers retain exactly the same
layout, physical capacity, rotation parameters, and absolute layer indices.
Each non-KV layer receives two one-F32 placeholders, so downstream absolute
indexing is unchanged and no KV kernel can consume a different numeric value.

For the canonical 64-layer hybrid shape with 16 FullAttention layers, four KV
heads, `head_dim=256`, and physical capacity 2,048, allocation-derived records
give 272.0 MiB Q8 or 186.0 MiB Asym3 when all 64 layers receive buffers. The
filtered production path uses 68.0 MiB or 46.5 MiB plus 384 bytes of
placeholders, reducing the cache allocation by approximately 75% (204.0 MiB
Q8 or 139.5 MiB Asym3 at this capacity).

A static audit pins mask derivation, both filtered constructor calls, shared
checked allocation, and absence of the unfiltered primary constructors. The
full locked workspace all-target suite, workspace examples, clean-room
audits, eleven-cell quant parity battery, four available standard coherence
cells, four DFlash/DDTree cells, and the Qwen3.6 27B agentic cell pass on
GPU0. Manual review found coherent bounded output, valid tool-call JSON, and
no speculative or agentic warning.

Three fresh GPU0 performance processes measure 1,299.1, 1,275.3, and 1,302.9
prefill tok/s and 140.9, 140.5, and 140.0 decode tok/s. Their medians are
1,299.1 and 140.5 tok/s. Relative to M13 they change by +0.27% and -0.14%, so
the 5% cross-batch expansion rule did not fire. Relative to the committed
W7900 floor they change by +5.85% and -0.28%.

The conservative direction-table position is now complete through row 55 of
2706; row 56 is next. The remaining direction count is 2651. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M15 evidence

### Checked hybrid KV capacity record

Direction row 56 records the preceding context-capacity reduction so its
evidence remains reproducible rather than manually transcribed. Commit
`8f34985` adds a GPU-free `hybrid_packed_kv_footprint` record derived from the
same checked `packed_kv_footprint` layout used by the constructors. It reports
all-layer bytes, filtered full-buffer bytes, non-KV placeholder bytes, and
saved bytes, while rejecting zero, inverted, and overflowing layer shapes.

The deterministic example now emits the four base packed formats plus Q8 and
Asym3 records for the canonical 64-layer Qwen3.5 hybrid shape. The exact Q8
record is 285,212,672 bytes all-layer, 71,303,552 bytes filtered, and
213,909,120 bytes saved. The Asym3 record is 195,035,136 bytes all-layer,
48,759,168 bytes filtered, and 146,275,968 bytes saved. Both include the exact
384 bytes of absolute-index placeholders. Unit tests pin these values and the
static audit separately requires four base records and two hybrid records.

The full locked workspace all-target suite, workspace examples, clean-room
audits, eleven-cell quant parity battery, four available standard coherence
cells, four DFlash/DDTree cells, and the Qwen3.6 27B agentic cell pass on
GPU0. Manual review found coherent bounded output, valid tool-call JSON, and
no speculative or agentic warning.

Three fresh GPU0 performance processes measure 1,287.6, 1,284.5, and 1,293.9
prefill tok/s and 140.7, 140.6, and 139.8 decode tok/s. Their medians are
1,287.6 and 140.6 tok/s. Relative to M14 they change by -0.89% and +0.07%, so
the 5% cross-batch expansion rule did not fire. Relative to the committed
W7900 floor they change by +4.91% and -0.21%.

The conservative direction-table position is now complete through row 56 of
2706; row 57 is next. The remaining direction count is 2650. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M16 evidence

### Uploaded-position reuse for long-context Q8

Direction row 57 explores reduced context-path movement. Commit `3a00613`
removes three loop-local four-byte allocations and per-row host-to-device
position uploads from the Q8 prefill fallback above 15,000 tokens. Plain
LLaMA and both Qwen3.5 FullAttention branches now pass one-element,
non-owning views into the batch position array that was already uploaded.
The position bits, sequence lengths, Q/K/V buffers, and flash kernels remain
unchanged. With the capture-illegal operations gone, the plain path no longer
rejects long-context Q8 graph capture.

A GPU0 0.8B Q8-cache probe forced the path with 15,001 input tokens. Prefill
completed in 4,519.2 ms and the subsequent decode captured 338 HIP graph
blobs. At sequence length 15,006, flash attention measured 166.3 us versus
929.7 us for the reference kernel (5.59x), with maximum absolute output delta
`5.25e-6`. A static audit pins all three device views and rejects recurrence
of the temporary buffer or stale capture guard.

The complete workspace, audits, eleven-cell quant parity, standard coherence,
DFlash/DDTree, and agentic gates pass on GPU0. Three fresh performance
processes measure 1,266.5, 1,285.7, and 1,286.1 prefill tok/s and 140.4, 140.1,
and 139.9 decode tok/s. Medians are 1,285.7 and 140.1 tok/s, changing by
-0.15% and -0.36% from M15; the 5% expansion rule did not fire.

The conservative direction-table position is now complete through row 57 of
2706; row 58 is next. The remaining direction count is 2649. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M17 evidence

### Machine-readable long-context Q8 record

Direction row 58 records the preceding context-path exploration. Commit
`885f45c` extends the existing GPU benchmark with `--record PATH` and emits
the versioned `hipfire.long_context_q8.v1` JSON schema. The record binds the
model and GPU architecture to prefill length/time, measured sequence length,
reference and flash attention latency, speedup, and absolute/relative parity.
A static audit requires every field to remain derived from the values printed
by the production benchmark.

The canonical GPU0 record uses the 0.8B model and 15,001-token Q8 prefill. It
reports 4,574.5 ms prefill, sequence length 15,006, 921.9 us reference
attention, 168.8 us flash attention, 5.46x speedup, and maximum absolute delta
`5.72e-6`. Automated JSON checks require the schema, >15K coverage, flash
speedup above one, and maximum absolute delta below `1e-3`.

The full workspace and MIT audits plus all GPU0 quality gates pass. Three
fresh performance processes measure 1,319.6, 1,261.4, and 1,274.8 prefill
tok/s and 141.2, 140.5, and 139.9 decode tok/s. Medians are 1,274.8 and 140.5
tok/s, changing by -0.85% and +0.29% from M16, so no five-run expansion was
required.

The conservative direction-table position is now complete through row 58 of
2706; row 59 is next. The remaining direction count is 2648. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M18 evidence

### Capturable Q8 long-context prefill

Direction row 59 continues reducing long-context state movement. Commit
`84b5d79` removes the obsolete Q8 >15K capture refusal after M16 made the
per-position fallback allocation-free. It also routes the final hidden-state
device copy through the capture-aware stream helper, avoiding an implicit
legacy-stream dependency while a blocking stream is being captured. A new
optional benchmark probe warms, captures, launches, and synchronizes the
production Qwen3.5 single-chunk prefill entry point, and records the captured
blob count in the existing versioned JSON evidence.

The canonical GPU0 probe uses a 15,001-token Q8 prefill followed by a two-token
captured prefill at position 15,001. The capture contains 447 HIP graph blobs
and launches successfully. The full run reports 4,610.4 ms initial prefill,
sequence length 15,006, 926.5 us reference attention, 168.4 us flash attention,
5.50x speedup, and maximum absolute delta `6.44e-6`.

The full workspace and MIT audits, eleven-cell quant parity, standard
coherence, four DFlash/DDTree cells, and the Qwen3.6 27B agentic cell all pass
on GPU0. Manual review found coherent bounded output, clean speculative
detectors, valid tool-call JSON, and zero agentic warnings. Three fresh
performance processes measure 1,295.8, 1,289.9, and 1,268.7 prefill tok/s and
140.9, 140.5, and 140.4 decode tok/s. Medians are 1,289.9 and 140.5 tok/s,
changing by +1.18% and 0.00% from M17, so no five-run expansion was required.

The conservative direction-table position is now complete through row 59 of
2706; row 60 is next. The remaining direction count is 2647. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M19 evidence

### Fail-closed architecture-family configuration

Direction row 60 is a model-adapter correctness fix. Commit `b3ab919` makes
the Qwen3.5, LLaMA/plain-Qwen, and Qwen3.5-VL adapters enforce their declared
HFQ architecture families before parsing metadata. A caller can therefore no
longer invoke a valid-looking config parser through the wrong adapter and
silently obtain a configuration for an unsupported family. Diagnostics state
the observed ID and the adapter's accepted IDs. The checks remain in the
cold-path adapter boundary and do not alter forward kernels or model math.

A dedicated audit pins all three production adapters to the same fail-closed
contract. The complete workspace, MIT checks, eleven-cell quant parity,
standard coherence, four DFlash/DDTree cells, and Qwen3.6 27B agentic cell
pass on GPU0. Manual review found coherent output, clean speculative
detectors, valid tool-call JSON, and zero agentic warnings.

Three fresh performance processes measure 1,309.9, 1,312.2, and 1,281.1
prefill tok/s and 140.9, 140.3, and 139.8 decode tok/s. Medians are 1,309.9
and 140.3 tok/s, changing by +1.55% and -0.14% from M18, so no five-run
expansion was required.

The conservative direction-table position is now complete through row 60 of
2706; row 61 is next. The remaining direction count is 2646. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M20 evidence

### Adapter-owned protocol labels

Direction row 61 expands the model-adapter boundary. Commit `de39757` adds a
fail-closed `protocol_label` hook to the shared architecture contract and
moves dense/MoE Qwen3.5 and LLaMA/plain-Qwen label ownership into their
adapters. The daemon now performs only generic adapter selection and no longer
duplicates the `arch_id = 6` variant rule. Unit tests pin every existing label
and the unknown-ID result, so this refactor preserves the wire protocol.

The full workspace, MIT checks, eleven-cell quant parity, standard coherence,
four DFlash/DDTree cells, and Qwen3.6 27B agentic cell pass on GPU0. Manual
review found coherent output, clean speculative detectors, valid tool-call
JSON, and zero agentic warnings. Three fresh performance processes measure
1,301.5, 1,270.5, and 1,314.5 prefill tok/s and 141.2, 140.8, and 140.3 decode
tok/s. Medians are 1,301.5 and 140.8 tok/s, changing by -0.64% and +0.36%
from M19, so no five-run expansion was required.

The conservative direction-table position is now complete through row 61 of
2706; row 62 is next. The remaining direction count is 2645. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M21 evidence

### Stop-before-emit speculative generation

Direction row 62 fixes user-visible generation semantics. Commit `52b5934`
makes the daemon's DFlash/DDTree path classify the prefill result and every
batched committed token with the shared model/tokenizer/frame stop contract
before adding it to token events or decoded text. A terminating first token
also bypasses the speculative loop. EOS, auxiliary EOT, and ChatML frame-stop
tokens therefore remain control flow rather than leaking into the user stream.
A static audit pins both ordering points and the first-token loop guard.

The full workspace, MIT checks, eleven-cell quant parity, standard coherence,
four DFlash/DDTree cells, and Qwen3.6 27B agentic cell pass on GPU0. Manual
review found coherent bounded output, clean speculative detectors, valid
tool-call JSON, and zero agentic warnings.

The first three fresh performance processes measured 1,394.6, 1,374.9, and
1,302.8 prefill tok/s, whose median differed by more than 5% from M20. Per the
cross-batch rule, two more independent GPU0 processes were run and measured
1,286.3 and 1,275.3 tok/s. The five-run prefill median is 1,302.8 tok/s.
Decode observations are 141.0, 140.8, 140.5, 140.1, and 140.2 tok/s, with a
140.5 tok/s median. Relative to M20 the final medians change only +0.10% and
-0.21%.

The conservative direction-table position is now complete through row 62 of
2706; row 63 is next. The remaining direction count is 2644. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.

## M22 evidence

### Adapter-owned PFlash family checks

Direction row 63 continues isolating model-family differences behind the
architecture adapter. Commit `b0d8ebc` removes the PFlash loader's duplicated
Qwen3.5 architecture-ID comparison in both the production path and its load
demo. They now ask the Qwen3.5 adapter whether the HFQ family is supported, so
future family changes have one owner. The architecture audit rejects a return
to hard-coded family checks in either consumer. This is a cold-path ownership
change and does not alter PFlash kernels, model math, or scheduling.

The recorded PFlash timing baseline requires a 27B MQ3 target that is not
present on this host. A GPU0 supplemental run therefore used the available
27B MQ4 target with the same 0.8B MQ4 drafter. All twelve fixture executions
returned PASS: all six historical PFlash PASS verdicts stayed PASS, and the
four baseline-mode historical FAIL verdicts improved to PASS. The gate's
mechanical exit was nonzero because it treats any verdict flip as a change and
because two MQ4 observations were more than 10% faster than the MQ3 timing
record. Those cross-format absolute timings are intentionally not accepted as
a performance comparison and the committed MQ3 baseline was not modified.

The full workspace, MIT checks, eleven-cell quant parity, standard coherence,
four DFlash/DDTree cells, and Qwen3.6 27B agentic cell pass on GPU0. Manual
review found coherent bounded output, clean speculative detectors, valid
tool-call JSON, and zero agentic warnings. Three fresh performance processes
measure 1,298.8, 1,327.1, and 1,362.2 prefill tok/s and 140.2, 139.3, and
139.8 decode tok/s. Medians are 1,327.1 and 139.8 tok/s, changing by +1.87%
and -0.50% from M21, so no five-run expansion was required.

The conservative direction-table position is now complete through row 63 of
2706; row 64 is next. The remaining direction count is 2643. Direction rows
remain audit inputs aggregated into independently specified implementation
batches, not a promise of one Git commit per row.
