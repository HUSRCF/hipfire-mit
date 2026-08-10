<!-- SPDX-License-Identifier: MIT -->
# Clean-room performance record

Copy this section for each performance-sensitive milestone.

## Run identity

- Date:
- Baseline commit:
- Candidate commit:
- GPU and gfx target:
- ROCm/HIP version:
- Benchmark binary md5:
- Prompt path:
- Prompt md5:
- Model path:
- Model md5:
- Exact command and relevant `HIPFIRE_*` environment:

## Measurements

| Variant | Warm-up | Run 1 | Run 2 | Run 3 | Median |
|---|---:|---:|---:|---:|---:|
| Baseline decode tok/s | | | | | |
| Candidate decode tok/s | | | | | |
| Baseline prefill tok/s | | | | | |
| Candidate prefill tok/s | | | | | |
| Baseline DFlash tok/s / tau | | | | | |
| Candidate DFlash tok/s / tau | | | | | |

## Decision

- Median delta:
- Delta at least 5% investigated:
- Correctness gate:
- Coherence report:
- Decoded output visually checked:
- Accept, revise, or reject:

---

## M0: Radeon Pro W7900 baseline — 2026-08-09

### Run identity

- Baseline commit: `d657b7ca27dc00f1f406dde046a76c64372278b6`
- Candidate: M0 working tree; inference and kernel paths are unchanged.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0.
- Board clocks reported by sysfs: SCLK 1760 MHz, MCLK 1124 MHz.
- ROCm/HIP: `7.14.60850-0000000`.
- Benchmark binary md5: `b9ee77210d15b23228f49cabb6f19cb7`.
- Prompt: deterministic synthetic token IDs `0..31`; tokenizer and prompt
  bytes are not involved.
- Model: `/home/husrcf/.hipfire/models/qwen3.5-4b.mq4` (read-only).
- Model md5: `712b69f8cf1016081cfa507c4d50e33d`.
- Environment: `HIPFIRE_KV_MODE=asym3`, `HIPFIRE_GRAPH=1`,
  `HIPFIRE_DPM_WARMUP_SECS=3`.
- Command: `bench_qwen35_mq4 MODEL --prefill 32 --warmup 5 --gen 50`.

### Measurements

| Metric | Run 1 | Run 2 | Run 3 | Median |
|---|---:|---:|---:|---:|
| Prefill tok/s | 1094.1 | 1227.3 | 1280.9 | 1227.3 |
| Decode tok/s | 141.5 | 140.9 | 140.8 | 140.9 |
| Effective decode GiB/s | 341.0 | 339.6 | 339.3 | 339.6 |

### Decision

- The existing `gfx1100` floor was captured on an RX 7900 XTX and remains
  unchanged.
- Comparing the W7900 against the XTX floor produced a false board-mismatch
  failure: prefill was +21.8%, while decode was -20.4%.
- A separate `gfx1100-w7900` profile now protects this board without lowering
  the XTX performance requirement.
- M0 changes do not touch kernels, dispatch, forward pass, or speculative
  decode. The short coherence battery was nevertheless run and passed with
  no hard errors; report: `/tmp/coherence-20260809-230336.md`. All four
  decoded outputs were visually reviewed as fluent and on-topic with no
  attractor loop or special-token corruption.
- Decision: accept M0 and use this profile as the baseline for subsequent
  clean-room implementation on this machine.

---

## M1: greedy speculative-verification contract — 2026-08-09

### Run identity

- Baseline commit: `78b6857e025354e70f03e6f941573667670037a7`.
- Candidate commit: `339756698413223894d4114427764773dd4428b3` (built from
  the staged tree immediately before that commit).
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Baseline benchmark binary md5: not captured before the candidate rebuild;
  the source commit and all other run inputs are fixed below. A reconstructed
  binary hash is deliberately not substituted for the measured artifact.
- Candidate benchmark binary md5: `070bce26532b5f29fcf520adb4912dc3`.
- Prompt: `benchmarks/prompts/merge_sort_thinking_off.txt`.
- Prompt md5: `253c7ac50857fe6d0e10fb0d2c5e35c0`.
- Target: `/home/husrcf/.hipfire/models/qwen3.5-9b.mq4` (read-only).
- Target md5: `296092bf1e6a45d78c1acf815eb93366`.
- Draft: `/home/husrcf/.hipfire/models/qwen35-9b-dflash-mq4.hfq`
  (read-only).
- Draft md5: `590f35403cd7f1d634945233234a12b7`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_DPM_WARMUP_SECS=10`, and ROCm 7.14 runtime directories prepended
  to `LD_LIBRARY_PATH`.
- Command: `dflash_spec_demo --target TARGET --draft DRAFT --prompt-file
  benchmarks/prompts/merge_sort_thinking_off.txt --max 256 --no-chatml
  --kv-mode asym3`, run under `scripts/gpu-lock.sh` in a fresh process for
  every sample.
- The baseline's first-ever 32.66 tok/s process was excluded before the
  three-sample comparison because it performed one-time kernel JIT. No
  candidate process was selectively excluded.

### Measurements

| Metric | Baseline 1 | Baseline 2 | Baseline 3 | Baseline median | Candidate 1 | Candidate 2 | Candidate 3 | Candidate median |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Decode tok/s | 498.19 | 495.86 | 494.77 | 495.86 | 501.24 | 497.14 | 495.76 | 497.14 |
| τ (accepted draft tokens/cycle) | 13.1818 | 13.1818 | 13.1818 | 13.1818 | 13.1818 | 13.1818 | 13.1818 | 13.1818 |
| Emitted tokens | 157 | 157 | 157 | 157 | 157 | 157 | 157 | 157 |

### Decision

- Candidate median delta: +0.26%; below the 5% investigation threshold and
  not a performance regression.
- The deterministic acceptance output, emitted count, cycle count, accepted
  count, and τ remained unchanged. The displayed committed-token statistic
  was corrected from storage-framed 167 total / 15.182 mean to logical 156
  total / 14.182 mean (τ + one target bonus per cycle).
- Correctness: `cargo test --locked -p hipfire-arch-qwen35 --lib` passed all
  28 tests, including six new speculative-contract tests.
- Clean-room license/source gate: passed.
- GPU0 DFlash coherence: passed both fast 27B cells with no hard or soft
  detector warnings; report `/tmp/coherence-dflash-20260809-231452.md`.
- Decoded prose and code outputs were manually checked and accepted.
- Decision: accept M1 rows 3-8.

---

## M2: device-execution contract — 2026-08-09

### Run identity

- Baseline commit: `0ba62820e50c2b1ebb31e641e3009a44b95a2651`.
- Candidate commit: `793c5b9b3126bf443bafe5ee83d0c1e59ef23dce`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`; live initialization reported HIP 7.14
  and 48.3 GB VRAM.
- Candidate benchmark binary md5:
  `afbfedb8d5b868e782927827ee407fe5`.
- Prompt: deterministic synthetic token IDs `0..31`; tokenizer and prompt
  bytes are not involved.
- Model: `/home/husrcf/.hipfire/models/qwen3.5-4b.mq4` (read-only).
- Model md5: `712b69f8cf1016081cfa507c4d50e33d`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`,
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`, and ROCm 7.14 runtime directories
  prepended to `LD_LIBRARY_PATH`.
- Command: `./scripts/speed-gate.sh --fast --verbose`, serialized by
  `scripts/gpu-lock.sh`. Each observation below is the gate's best of two
  fresh benchmark executions. Observation 1 preceded the commit, observation
  2 ran inside the commit hook, and observation 3 ran after the commit.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1294.3 | 1310.5 | 1311.0 | 1310.5 |
| 4B MQ4 decode tok/s | 140.9 | 141.5 | 141.3 | 141.0 | 141.3 |

### Decision

- Candidate median delta: +6.78% prefill and +0.28% decode. Neither metric
  regressed; all three gate observations passed the 5% floor.
- Live GPU0 smoke: the runtime resolved device 0 as `gfx1100`, loaded the 4B
  model, and completed one prefill and one decode token.
- Correctness: `cargo test --locked -p rdna-compute --lib` passed all four
  target-contract tests; `scripts/verify-bind-thread.sh` audited 380 public
  entries and passed both single- and multi-device invariants.
- Clean-room license/source gate: passed.
- Coherence: the post-commit-hook report
  `/tmp/coherence-20260809-234600.md` passed all four available cells. Outputs
  were manually reviewed as coherent and on-topic with no attractor or
  special-token corruption.
- Agentic A/B diagnostic: candidate daemon md5
  `2baa852590a46f03ca85ec5bd5151193`; detached-parent daemon md5
  `032972644846276187775c751eb8b727`. Both produced the same malformed
  `name` field for the fixed Qwen 3.6 27B tool-call cell; the candidate did so
  twice. Reports: `/tmp/agentic-m2-candidate-rerun.md` and
  `/tmp/agentic-m2-parent-0ba6282.md`. This pre-existing failure was isolated
  from the device change and remains an open correctness item.
- Decision: accept the device-execution foundation; do not mark all M2 rows
  complete until the remaining format, maintenance, integration, and agentic
  correctness work is closed.

---

## M2 follow-up: agentic JSON structural contract — 2026-08-09

### Run identity

- Regression parent: `3a241b1`; the earlier detached `0ba6282` A/B established
  that the malformed response predated the intervening device-only changes.
- Structural fix: `76dd2b97bd83ab61839f7bf02988f41c49f8c2fb`.
- Gate-coverage fix: `658baa8a7fd5e2f45fd4ffadd7577e89391702ba`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate daemon md5: `d816324399c294e26f5915d194da51d7`.
- Candidate benchmark binary md5:
  `a2442c9857978cbbdb6f9d6826ddedff`.
- Agentic target models:
  `/home/husrcf/.hipfire/models/qwen3.6-27b.mq4` and
  `/home/husrcf/.hipfire/models/qwen3.5-35b-a3b.mq4` (read-only).
- Agentic inputs: `benchmarks/prompts/agentic_pi_system.txt`,
  `benchmarks/prompts/agentic_hermes_system.txt`, and
  `benchmarks/prompts/agentic_user_read.txt`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`, and ROCm 7.14 runtime
  directories prepended to `LD_LIBRARY_PATH`.
- Quality commands: `./scripts/agentic-gate.sh --fast` and the full
  `./scripts/agentic-gate.sh`, serialized by `scripts/gpu-lock.sh`.
- Performance command: `./scripts/speed-gate.sh --fast --verbose` with
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`. Each observation is the gate's best
  of two benchmark executions.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1351.6 | 1320.6 | 1309.8 | 1320.6 |
| 4B MQ4 decode tok/s | 140.9 | 142.5 | 142.1 | 141.8 | 142.1 |

### Decision

- Candidate median delta: +7.60% prefill and +0.85% decode. All three
  observations pass the committed floor.
- Correctness: all 164 `hipfire-runtime` library tests pass, including the
  valid, malformed-name, unrelated-output, and closed-block constraint cases.
  The daemon example compiles with the `deltanet` feature, the agentic detector
  self-check fires all four predicates, and the clean-room gate passes.
- Fast GPU0 quality: PASS, zero hard failures and zero soft warnings. The raw
  Qwen 3.6 27B response contains valid JSON with `name='read'` and an
  `arguments` object. Post-commit report:
  `/tmp/agentic-76dd2b9-postcommit.md`.
- Full GPU0 quality: all eight cells complete with zero structural hard
  failures. Four Qwen 3.6 cells and two Qwen 3.5 Pi cells pass. Two Qwen 3.5
  Hermes cells soft-warn because the model emits `<think><|im_end|>` without a
  tool call, so the aggregate threshold still returns failure. Report:
  `/tmp/agentic-m2-quote-constraint-full.md`.
- Decision: accept the bounded JSON structural fix and keep the 35B Hermes
  tool-selection reliability item open; do not weaken the full-gate threshold.

---

## M2 follow-up: empty-thinking termination recovery — 2026-08-10

### Run identity

- Regression parent: `511d4ad`.
- Candidate commit: `cab7bb2e2207cc42d70bfbe7db6bb193e6436b5d`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate daemon md5: `6ec11cd7b3b577ce143a2d0d0412ccd6`.
- Candidate benchmark binary md5:
  `af5c380e3c3a8becec23f7e4348646f3`.
- Agentic target models:
  `/home/husrcf/.hipfire/models/qwen3.6-27b.mq4` and
  `/home/husrcf/.hipfire/models/qwen3.5-35b-a3b.mq4` (read-only).
- Agentic inputs: `benchmarks/prompts/agentic_pi_system.txt`,
  `benchmarks/prompts/agentic_hermes_system.txt`, and
  `benchmarks/prompts/agentic_user_read.txt`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`, and ROCm 7.14 runtime
  directories prepended to `LD_LIBRARY_PATH`.
- Targeted quality commands: `HIPFIRE_AGENTIC_FAST_MODEL=3.5
  ./scripts/agentic-gate.sh --fast` and the corresponding `3.6` command.
  Full quality command: `./scripts/agentic-gate.sh`. Every GPU command was
  serialized by `scripts/gpu-lock.sh`.
- Performance command: `./scripts/speed-gate.sh --fast --verbose` with
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`. Each observation is the gate's best
  of two benchmark executions. Observation 1 preceded the commit, observation
  2 ran inside the commit hook, and observation 3 ran after the commit.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1286.6 | 1295.1 | 1288.8 | 1288.8 |
| 4B MQ4 decode tok/s | 140.9 | 141.6 | 141.8 | 140.7 | 141.6 |

### Decision

- Candidate median delta: +5.01% prefill and +0.50% decode. All three gate
  observations pass the committed 5% regression tolerance.
- Correctness: all 166 `hipfire-runtime` library tests pass, including the
  terminator, non-terminator, non-empty, already-closed, and plain-output
  constraint cases. The daemon example and benchmark were rebuilt from the
  candidate source, the agentic detector self-check passed, and the
  clean-room source/license gate passed.
- Targeted GPU0 quality: both the 35B and 27B fast cells pass with valid
  `name` and `arguments` JSON and zero warnings. Reports:
  `/tmp/agentic-empty-think-35b-fast.md` and
  `/tmp/agentic-empty-think-36-fast.md`.
- Full GPU0 quality: all eight cells pass with zero hard failures and zero
  soft warnings. Every cell emitted a parseable tool call; report:
  `/tmp/agentic-empty-think-full.md`.
- Commit-hook quality: the Qwen 3.6 agentic cell passed again with zero
  warnings. The four-cell short coherence battery had no hard errors and no
  token corruption or repetition loop. Its 9B reasoning response reached the
  configured output cap while remaining coherent. Reports:
  `/tmp/agentic-gate-20260810-004225.md` and
  `/tmp/coherence-20260810-004208.md`.
- Decision: accept the bounded empty-thinking recovery and close the 35B
  Hermes tool-selection reliability item without weakening the full gate.

---

## M3: HFQ file-boundary validation — 2026-08-10

### Run identity

- Regression parent: `28b5ca9`.
- Candidate commit: `6088d29a2bc932cbb4b38be3ac1bb2d1299e04a5`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate daemon md5: `fdd91a592cc126d291e3a83368e99c78`.
- Candidate DFlash binary md5: `065b82554eebc9b12736f6c553f20201`.
- Candidate benchmark binary md5:
  `e40e29f8a452441bf1d912d52eb6b429`.
- Compatibility files (read-only): Qwen 3.5 4B MQ4, Qwen 3.6 27B MQ3,
  Qwen 3.5 27B MQ4, and Qwen 3.5 9B/27B MQ4 DFlash under
  `/home/husrcf/.hipfire/models`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`, and ROCm 7.14 runtime
  directories prepended to `LD_LIBRARY_PATH`.
- Quality commands: `./scripts/coherence-gate.sh` and the full
  `./scripts/coherence-gate-dflash.sh`, serialized by
  `scripts/gpu-lock.sh`. The MQ3 smoke used the same lock and GPU binding.
- Performance command: `./scripts/speed-gate.sh --fast --verbose` with
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`. Each observation is the gate's best
  of two benchmark executions using deterministic synthetic token IDs
  `0..31`; tokenizer and prompt bytes are not involved.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1357.9 | 1391.4 | 1387.0 | 1387.0 |
| 4B MQ4 decode tok/s | 140.9 | 141.1 | 141.1 | 139.9 | 141.1 |

The commit-hook repetition also passed at 1279.3 prefill tok/s and 141.3
decode tok/s.

### Decision

- Candidate median delta: +13.01% prefill and +0.14% decode; all observations
  pass the committed regression tolerance. The parser executes before the
  benchmark's inference timer, so the elevated prefill median is not claimed
  as a code-caused performance gain.
- Correctness: all 172 `hipfire-runtime` library tests pass. The six new HFQ
  tests exercise valid parsing and malformed headers, versions, offsets,
  metadata, counts, names, indices, and payload ranges without GPU access.
- Compatibility: CPU index reads pass for qt=3 MQ4 embeddings, qt=17 MQ3
  output weights, and qt=13 MQ4 DFlash weights. A Qwen 3.6 27B MQ3 GPU0 smoke
  loaded 64 layers, captured its graph, emitted five tokens without a runtime
  error, and unloaded cleanly. Its very short `The` answer is recorded only as
  execution compatibility, not as a quality pass.
- Standard GPU0 coherence: no hard errors or repetition loops; report
  `/tmp/coherence-20260810-005247.md`. Two capped responses were incomplete
  but remained on-topic and structurally valid. The commit-hook rerun also
  passed; report `/tmp/coherence-20260810-005913.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; prose and code outputs were manually checked for
  attractors and structural repetition. Report:
  `/tmp/coherence-dflash-20260810-005329.md`.
- Agentic commit-hook quality: PASS with valid `name='read'` JSON and zero
  warnings; report `/tmp/agentic-gate-20260810-005940.md`.
- Decision: accept the strict HFQ boundary contract as the first M3
  deliverable. Numerical quantization fidelity and cache-path milestones
  remain open.

## M3: packed KV allocation contract — 2026-08-10

### Run identity

- Regression parent: `a7afe07`.
- Runtime candidate commit:
  `c3dfe24fd2734cfef21f30b2cac6e7f3daab5d8d`.
- Benchmark-freshness follow-up:
  `217645bef42d1ca79e3bf4ab4a35c61b96785e20`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate daemon md5: `16eaa9cf4d97c6e2243ff6daaa610015`.
- Candidate DFlash binary md5: `e4a88c2314c547a0a1a4cc0ad3cad694`.
- Explicitly rebuilt candidate benchmark md5:
  `bf1c2381938316a295487c5984291d82`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`,
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`, and ROCm 7.14 runtime directories
  prepended to `LD_LIBRARY_PATH`.
- Correctness commands: `cargo test -p hipfire-runtime --lib`,
  `./scripts/cleanroom-gate.sh`, `./scripts/coherence-gate.sh`, and
  `./scripts/coherence-gate-dflash.sh`.
- Performance command: `./scripts/speed-gate.sh --fast --verbose`. The
  benchmark was explicitly rebuilt before the three recorded observations;
  each observation is still the gate's best of two executions using its
  deterministic synthetic token IDs `0..31`.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1308.3 | 1262.2 | 1288.1 | 1288.1 |
| 4B MQ4 decode tok/s | 140.9 | 141.0 | 140.6 | 140.4 | 140.6 |

After the freshness fix, an additional end-to-end gate run passed at 1259.2
prefill tok/s and 140.7 decode tok/s while first invoking Cargo's current-
target check.

### Decision

- Candidate median delta: +4.95% prefill and -0.21% decode. Every observation
  passes the committed 5% regression tolerance. The runtime change performs
  checked layout arithmetic only during cache construction, so these values
  are recorded as non-regression and no speedup is attributed to it.
- Correctness: all 177 `hipfire-runtime` library tests pass. Five new tests
  pin Q8 and packed 2/3/4-bit per-head strides, F32 allocation rounding,
  physical-cap scaling, invalid dimensions/capacities, and host-size overflow.
  The clean-room SPDX/source gate and `git diff --check` pass.
- Standard GPU0 coherence: four available cells complete without hard errors;
  outputs were manually checked for fluency, task relevance, tool-call JSON,
  and repetition. Reports: `/tmp/coherence-kvlayout.md` and commit-hook rerun
  `/tmp/coherence-20260810-011300.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; prose and code were manually checked. Report:
  `/tmp/coherence-dflash-kvlayout.md`.
- Agentic commit-hook quality: one Qwen 3.6 27B cell passes with valid
  `name='read'` JSON and zero warnings. Report:
  `/tmp/agentic-gate-20260810-011324.md`.
- Evidence hygiene: three earlier speed-gate prechecks are excluded because
  their existing benchmark executable still had md5
  `e40e29f8a452441bf1d912d52eb6b429`, predating the runtime candidate. Commit
  `217645bef42d1ca79e3bf4ab4a35c61b96785e20` closes that harness gap by always
  asking Cargo to validate target freshness and by building the DFlash target
  as well in non-fast mode.
- Decision: accept the shared packed-KV allocation contract as the second M3
  deliverable. Numerical cache fidelity under long-context eviction and the
  remaining quantization-format milestones remain open.

---

## M3: long-context eviction position continuity — 2026-08-10

### Run identity

- Regression parent: `0f7b4a1`.
- Candidate commit: `4c2733d99dcf7fda94f22d4b449ba06837c50f07`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `2b127348de60a6dce7c412a7d7b03641`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- Candidate TriAttention long-context md5:
  `ab9caa836c9a00ecf2aa1c9c22c0ae92`.
- Candidate CASK inference md5: `4abf326e5d2216dfd0315850981b4d58`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`,
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`, and ROCm 7.14 runtime directories
  prepended to `LD_LIBRARY_PATH`. Every GPU command was serialized by
  `scripts/gpu-lock.sh`.
- Correctness commands: `cargo test -p hipfire-runtime --lib`,
  `cargo check -p hipfire-runtime --examples --features deltanet`,
  `./scripts/cleanroom-gate.sh`, `./scripts/coherence-gate.sh`, and
  `./scripts/coherence-gate-dflash.sh`.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`. Each observation is the gate's best of two
  executions using deterministic synthetic token IDs `0..31`.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1369.0 | 1291.1 | 1261.3 | 1291.1 |
| 4B MQ4 decode tok/s | 140.9 | 141.0 | 140.4 | 139.8 | 140.4 |

### Decision

- Candidate median delta: +5.20% prefill and -0.35% decode. All three runs
  pass the committed 5% regression tolerance. The changed policy is opt-in
  and absent from this benchmark, so no implementation-caused speedup is
  claimed.
- Correctness: all 181 `hipfire-runtime` library tests pass, including four
  new pure tests for valid and invalid schedules, repeated position
  continuity, capacity mismatch, and host-offset overflow. Every runtime
  example compiles with DeltaNet enabled, and the clean-room gate passes.
- Plain eviction: the Qwen 3.5 9B asym3 long-context demo processed a
  122-token prompt and 32 generated tokens with a fixed 42-position cache,
  completing 15 evictions. After prefill it reported `physical=34` and
  `compact_offset=88`; the process exited successfully.
- CASK eviction: the Qwen 3.5 9B Q8 run used `budget=32`, `beta=8`,
  `core_frac=0.5`, and `fold_m=2`, completing five evictions and ending with
  `compact_offset=40`. Its generated response remained coherent.
- The temporary 9B sidecar was saved after 64 calibration tokens solely to
  exercise the eviction dispatches. The calibration tool's later validation
  phase hit the existing HIP stream-capture error 906, so neither its reported
  MRL nor downstream sidecar quality is treated as evidence.
- Standard GPU0 coherence: four available cells complete without hard errors;
  outputs were manually checked for task relevance, valid tool JSON, and
  repetition. Report: `/tmp/coherence-eviction-position.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; prose and code outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-eviction-position.md`.
- The two coherence reports identify parent `0f7b4a1` because validation ran
  before the source commit; their binaries were freshly rebuilt from the
  candidate working tree recorded above.
- Decision: accept checked repeated-eviction position continuity as the third
  M3 deliverable. Quantization numerical-fidelity and the remaining format
  milestones stay open.

---

## M3: HFQ quant-payload layout validation — 2026-08-10

### Run identity

- Regression parent: `29cf4363b5cf3bd5cf8b334358067302d56110de`.
- Candidate commit: `ab1163dcd08ada578e2a9f38aaf71a90396c7b7e`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `5bc7854e8530d4c1c75f0ba1aab2aa98`.
- Candidate daemon md5: `8b3cd494a24a7378ddaf38be58c88ae6`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`,
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`, and ROCm 7.14 runtime directories
  prepended to `LD_LIBRARY_PATH`. Every GPU command was serialized by
  `scripts/gpu-lock.sh`.
- Correctness commands: `cargo test -p hipfire-runtime --lib`,
  `cargo check -p hipfire-runtime --examples --features deltanet`,
  `./scripts/cleanroom-gate.sh`, `./scripts/coherence-gate.sh`, and
  `./scripts/coherence-gate-dflash.sh`.
- Compatibility check: the release `query_tensor` example parsed the indices
  of 20 complete local model files. Incomplete, temporary, lock, and known-bad
  files were excluded explicitly.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`. Each observation is the gate's best of two
  executions using deterministic synthetic token IDs `0..31`.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1290.6 | 1324.6 | 1271.7 | 1290.6 |
| 4B MQ4 decode tok/s | 140.9 | 140.9 | 139.9 | 140.0 | 140.0 |

The commit-hook repetition also passed at 1294.1 prefill tok/s and 141.6
decode tok/s.

### Decision

- Candidate median delta: +5.16% prefill and -0.64% decode. Every observation
  passes the committed 5% regression tolerance. Layout validation executes
  once while parsing the model index, outside the benchmark timer, so no
  implementation-caused speedup is claimed.
- Correctness: all 183 `hipfire-runtime` library tests pass. The layout table
  test covers every active quantization identifier; rejection cases cover
  wrong groups, short dense payloads, invalid Q8HFQ and FP4 shapes, and a
  reserved identifier. Runtime examples with DeltaNet and the clean-room
  source/license gate pass.
- Compatibility: 20 complete local HFQ indices pass across Qwen 3/3.5/3.6,
  Laguna, MQ3, MQ4, MQ4P, MQ4R, HF4, MTP, and DFlash files. The parser checks
  metadata and declared ranges only and does not scan tensor payload bytes.
- Standard GPU0 coherence: four available cells complete without hard
  errors; all outputs were manually checked for relevance, structure, and
  repetition. Report: `/tmp/coherence-hfq-layout.md`. The commit-hook rerun
  also passed: `/tmp/coherence-20260810-013928.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; prose and code outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-hfq-layout.md`.
- Agentic commit-hook quality: one Qwen 3.6 27B cell passes with valid tool
  JSON and zero warnings. Report: `/tmp/agentic-gate-20260810-013945.md`.
- Decision: accept exact registered quant-payload layout validation as the
  fourth M3 deliverable. Numerical quantization fidelity and the remaining
  format milestones stay open.

---

## M3: quantization numerical parity gate — 2026-08-10

### Run identity

- Regression parent: `acf2d0a9310c693d0f2eef72390e86ce885aa511`.
- Candidate commit: `dce4648a97490426b13ed267a81b27ef327c5811`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `5bc7854e8530d4c1c75f0ba1aab2aa98`.
- Candidate daemon md5: `8b3cd494a24a7378ddaf38be58c88ae6`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`,
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`, and ROCm 7.14 runtime directories
  prepended to `LD_LIBRARY_PATH`. Every GPU command was serialized by
  `scripts/gpu-lock.sh`.
- Correctness commands: `bash -n scripts/quant-parity-gate.sh
  .githooks/pre-commit`, `./scripts/cleanroom-gate.sh`, and
  `./scripts/quant-parity-gate.sh`. The updated pre-commit hook then repeated
  the parity gate, standard coherence, agentic, and speed gates.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`. Each observation is the gate's best of two
  executions using deterministic synthetic token IDs `0..31`.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1306.5 | 1302.9 | 1289.9 | 1302.9 |
| 4B MQ4 decode tok/s | 140.9 | 141.1 | 140.4 | 140.2 | 140.4 |

The commit-hook repetition also passed at 1307.6 prefill tok/s and 141.3
decode tok/s.

### Decision

- Candidate median delta: +6.16% prefill and -0.35% decode. All observations
  pass the committed 5% regression tolerance. The change only adds a test
  script and hook mapping, and the measured inference executable hash is
  unchanged, so no implementation-caused speedup is claimed.
- Numerical parity: all five GPU0 cases pass. HFQ4 reports
  `gpu_cpu_err=0.000366` and `mmq_err=0.040617`; HFQ6 reports maximum error
  `0.000061`. MQ3-Lloyd, HFP4, and MFP4 exercise quad-clean and tail-group
  shapes through K=2048 and remain below their case-specific tolerances.
  Report: `/tmp/quant-parity-cleanroom.md`; commit-hook repetition:
  `/tmp/quant-parity-20260810-014647.md`.
- Standard GPU0 coherence: all four available cells pass and were manually
  checked for relevance, valid tool JSON, and repetition. Report:
  `/tmp/coherence-20260810-014648.md`.
- Agentic commit-hook quality: one Qwen 3.6 27B cell passes with valid
  `name='read'` JSON and zero warnings. Report:
  `/tmp/agentic-gate-20260810-014704.md`.
- DFlash/DDTree inference code and binary are unchanged from the immediately
  preceding full 4/4 pass recorded under HFQ payload-layout validation.
- Decision: accept a mandatory independent numerical oracle as the fifth M3
  deliverable. Broader format coverage and model-level quantization quality
  remain open.

---

## M3: expanded registered-format parity coverage — 2026-08-10

### Run identity

- Regression parent: `8bcd7760b0e8a0e0625fa08b13fbfb234064ba56`.
- Candidate commit: `912d193ade56fb41db940f01036b7f7a680c9329`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `5bc7854e8530d4c1c75f0ba1aab2aa98`.
- Candidate daemon md5: `8b3cd494a24a7378ddaf38be58c88ae6`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- Classic-format anchor md5: `7b211d421224af85dd90e390ea17b422`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`,
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`, and ROCm 7.14 runtime directories
  prepended to `LD_LIBRARY_PATH`. Every GPU command was serialized by
  `scripts/gpu-lock.sh`.
- Correctness commands: `cargo test -p hipfire-runtime --lib`,
  `cargo check -p hipfire-runtime --examples --features deltanet`,
  `./scripts/cleanroom-gate.sh`, `./scripts/quant-parity-gate.sh`,
  `./scripts/coherence-gate.sh`, and `./scripts/coherence-gate-dflash.sh`.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`. Each observation is the gate's best of two
  executions using deterministic synthetic token IDs `0..31`.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1287.2 | 1324.8 | 1365.4 | 1324.8 |
| 4B MQ4 decode tok/s | 140.9 | 141.8 | 141.2 | 141.0 | 141.2 |

The commit-hook repetition also passed at 1393.7 prefill tok/s and 141.3
decode tok/s.

### Decision

- Candidate median delta: +7.94% prefill and +0.21% decode. All observations
  pass the committed regression tolerance. The inference executable hashes
  are unchanged because this batch adds test coverage only; no speedup is
  attributed to it.
- Classic formats: synthetic Q4K/Q4F16/Q8 data require no model files. Maximum
  GPU/CPU absolute errors are `2.384186e-7` (Q4K), `4.808977e-4`
  (Q4F16-G32), `4.756898e-4` (Q4F16-G64), `9.499490e-8` (Q8_0), and
  `6.332994e-8` (Q8HFQ). Q4K exercises packed high scale/min bits and Q8HFQ
  verifies a 512-byte aligned row stride.
- Additional formats: rotated MQ3/MQ2 remain below `1e-3`, isolated FWHT is
  element-exact through K=1024, HFQ3 residual output is exact at K=256,
  1024, 4096, and 11008, and Q8 KV round-trip error is `0.012305` under its
  fixed `0.05` threshold.
- Unified GPU0 parity: all nine cases pass. Report:
  `/tmp/quant-parity-expanded.md`; commit-hook repetition:
  `/tmp/quant-parity-20260810-094909.md`.
- Standard GPU0 coherence: four available cells pass and were manually
  checked. Report: `/tmp/coherence-quant-expanded.md`; commit-hook repetition:
  `/tmp/coherence-20260810-094912.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; prose/code outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-quant-expanded.md`.
- Agentic commit-hook quality: one Qwen 3.6 27B cell passes with valid
  `name='read'` JSON and zero warnings. Report:
  `/tmp/agentic-gate-20260810-094928.md`.
- Decision: accept portable numerical parity for the currently exercised
  registered classic, HFQ, MQ, HFP4/MFP4, and Q8-cache paths as the sixth M3
  deliverable. Model-level quality and remaining registered formats stay open.

---

## M3: MQ6 common-loader closure — 2026-08-10

### Run identity

- Regression parent: `6d0ab235ec3d4a56a5763637fa7da13b71b37639`.
- Candidate commit: `a432e8694356e4c1993266f75d8096d051e1c9d2`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `2225075fb69cda29d06034e64982296b`.
- Candidate daemon md5: `dbc7a5fdf00c9038992933f29696150b`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- MQ parity anchor md5: `649d98c68ee9a3d26942a5c38b41568a`.
- Environment: `ROCR_VISIBLE_DEVICES=0`, `HIP_VISIBLE_DEVICES=0`,
  `HIPFIRE_MODELS_DIR=/home/husrcf/.hipfire/models`,
  `HIPFIRE_BASELINE_ARCH=gfx1100-w7900`, and ROCm 7.14 runtime directories
  prepended to `LD_LIBRARY_PATH`. Every GPU command was serialized by
  `scripts/gpu-lock.sh`.
- Correctness commands: `cargo test -p hipfire-runtime --lib`,
  `cargo check -p hipfire-runtime --examples --features deltanet`,
  `./scripts/cleanroom-gate.sh`, `./scripts/quant-parity-gate.sh`,
  `./scripts/coherence-gate.sh`, and `./scripts/coherence-gate-dflash.sh`.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`. Each observation is the gate's best of two
  executions using deterministic synthetic token IDs `0..31`.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1270.0 | 1269.6 | 1248.4 | 1269.6 |
| 4B MQ4 decode tok/s | 140.9 | 141.4 | 140.3 | 140.2 | 140.3 |

The commit-hook repetition also passed at 1254.1 prefill tok/s and 140.8
decode tok/s.

### Decision

- Candidate median delta: +3.45% prefill and -0.43% decode. Every observation
  passes the committed 5% regression tolerance. The loader change routes a
  previously rejected format to its existing kernel; no speedup is attributed
  to the change.
- MQ6 numerical parity: the independent CPU decoder and GPU rotate-plus-GEMV
  agree with maximum absolute error `6.103516e-5` at K=256,
  `2.441406e-4` at K=512, and `8.544922e-4` at K=1024, under the fixed
  `1e-3` limit. Standalone FWHT output is element-exact at all three shapes.
- Unified GPU0 parity: all nine cases pass. Reports:
  `/tmp/quant-parity-mq6-loader.md` and commit-hook repetition
  `/tmp/quant-parity-20260810-100100.md`.
- Standard GPU0 coherence: four available cells pass and were manually
  checked. Reports: `/tmp/coherence-mq6-loader.md` and commit-hook repetition
  `/tmp/coherence-20260810-100102.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; prose/code outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-20260810-095806.md`.
- Agentic commit-hook quality: the Qwen3.6 27B cell emits valid
  `name='read'` JSON with zero warnings. Report:
  `/tmp/agentic-gate-20260810-100118.md`.
- Decision: accept common-loader support and a portable numerical oracle for
  MQ6-G256 as the seventh M3 deliverable. The remaining registered formats
  and model-level quality milestones stay open.

---

## M3: compact HFQ and MQ2-Lloyd parity closure — 2026-08-10

### Run identity

- Regression parent: `9061dc936e839d56f56b3f8cd3b652f174380def`.
- Candidate commit: `73be35f32d3ab833b2bc058636fffa0b7374cd3c`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `2225075fb69cda29d06034e64982296b`.
- Candidate daemon md5: `dbc7a5fdf00c9038992933f29696150b`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- Compact-format anchor md5: `a9d8b6dcb8a712e581bf08b20d7c0e66`.
- Environment: GPU0-only visibility and the same ROCm 7.14 library,
  model-directory, W7900 baseline, and `scripts/gpu-lock.sh` serialization
  used by the preceding M3 runs.
- Correctness commands: `cargo test -p hipfire-runtime --lib`,
  `cargo check -p hipfire-runtime --examples --features deltanet`,
  `./scripts/cleanroom-gate.sh`, `./scripts/quant-parity-gate.sh`,
  `./scripts/coherence-gate.sh`, and `./scripts/coherence-gate-dflash.sh`.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`; each observation is best-of-two.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1304.4 | 1307.0 | 1289.6 | 1304.4 |
| 4B MQ4 decode tok/s | 140.9 | 140.3 | 139.9 | 139.2 | 139.9 |

The commit-hook repetition also passed at 1287.5 prefill tok/s and 140.6
decode tok/s.

### Decision

- Candidate median delta: +6.28% prefill and -0.71% decode. All observations
  pass the committed regression tolerance. Benchmark, daemon, and DFlash
  hashes are unchanged from the preceding implementation batch, so no
  implementation-caused speedup is claimed.
- New compact-format maximum errors through K=1024 are `8.544922e-4`
  (HFQ4-G128), `8.544922e-4` (HFQ2-G256), `7.324219e-4`
  (HFQ2-G128), `1.464844e-3` (HFQ3-G128), and `3.814697e-5`
  (MQ2-G256-Lloyd), all below the anchor's `2e-3` limit.
- Unified GPU0 parity: all ten cases pass. Reports:
  `/tmp/quant-parity-compact.md` and commit-hook repetition
  `/tmp/quant-parity-20260810-101112.md`.
- Standard GPU0 coherence: four available cells pass after manual review.
  Reports: `/tmp/coherence-compact.md` and commit-hook repetition
  `/tmp/coherence-20260810-101115.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-20260810-100827.md`.
- Agentic commit-hook quality: Qwen3.6 27B emits valid `name='read'` JSON with
  zero warnings. Report: `/tmp/agentic-gate-20260810-101131.md`.
- Decision: accept independent parity for the remaining compact HFQ GEMV and
  MQ2-Lloyd paths as the eighth M3 deliverable. Model-level format quality and
  unexercised fused/batched variants remain open.

---

## M3: MQ8 gfx11 execution and MQ4/MQ8 parity closure — 2026-08-10

### Run identity

- Regression parent: `221a2efc11644f11f7981db32e78936a39bbcc43`.
- Candidate commit: `a36f989635a531dbff5a5e61e7f3e0e416c7de0c`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `c4a3eb86f27b00a88a32a7e32361f64c`.
- Candidate daemon md5: `dbc7a5fdf00c9038992933f29696150b`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- Expanded MQ parity anchor md5: `125c2f207f04fdef32a95a29a41676be`.
- Environment: GPU0-only visibility and the same ROCm 7.14 library,
  model-directory, W7900 baseline, and `scripts/gpu-lock.sh` serialization
  used by the preceding M3 runs.
- Correctness commands: `cargo test --workspace --all-targets`,
  `cargo check --workspace --examples`, `./scripts/cleanroom-gate.sh`,
  `./scripts/quant-parity-gate.sh`, `./scripts/coherence-gate.sh`, and
  `./scripts/coherence-gate-dflash.sh`.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`; each observation is best-of-two.

### Measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1345.1 | 1314.1 | 1382.9 | 1345.1 |
| 4B MQ4 decode tok/s | 140.9 | 140.5 | 140.0 | 139.8 | 140.0 |

The commit-hook repetition also passed at 1320.3 prefill tok/s and 140.1
decode tok/s.

### Decision

- Candidate median delta: +9.60% prefill and -0.64% decode. All observations
  pass the committed 5% regression tolerance. Relative to the preceding
  testing batch's median, the changes are +3.12% and +0.07%, respectively,
  so no additional jitter runs were required. The default benchmark exercises
  MQ4, not MQ8; it establishes non-regression and no MQ8-caused speedup is
  claimed.
- gfx1100 compilation: the original direct signed `sdot4` call fails with
  `needs target feature dot1-insts`. The candidate uses gfx11/gfx12's
  generalized `sudot4(true, a, true, b, acc, false)` and keeps `sdot4` for
  earlier supported targets. This matches the locally installed ROCm 7.14
  signed-int8 inner-product interface.
- MQ8 numerical parity: maximum absolute errors are `1.525879e-5`,
  `6.103516e-5`, and `1.220703e-4` at K=256, 512, and 1024, below `1e-3`.
  MQ4 reports `7.629395e-5`, `3.051758e-4`, and `1.220703e-3`, below its
  explicit `2e-3` FP32-reduction budget. Standalone FWHT is element-exact.
- Unified GPU0 parity: all ten cases pass. Reports:
  `/tmp/quant-parity-mq4-mq8-pass.md` and commit-hook repetition
  `/tmp/quant-parity-20260810-102517.md`.
- Standard GPU0 coherence: four available cells pass after manual review.
  Reports: `/tmp/coherence-mq8-gfx11.md` and hook repetition
  `/tmp/coherence-20260810-102519.md`.
- DFlash/DDTree GPU0 coherence: all four cells report `ok=true` and
  `soft_warn=false`; prose and code outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-mq8-gfx11.md`.
- Agentic commit-hook quality: Qwen3.6 27B emits valid `name='read'` JSON with
  zero warnings. Report: `/tmp/agentic-gate-20260810-102536.md`.
- Decision: accept MQ8 execution on gfx11 and independent MQ8/MQ4 parity as
  the ninth M3 deliverable. Model-level format quality and fused/batched
  execution variants remain open.

---

## M3: MQ8 scalar-dot negative experiment — 2026-08-10

### Run identity

- Regression parent: `a8678c67ffd12f9134217ad2c972fdd86dac056c`.
- Candidate commit: `fcbc0489ac923e6bbc99f56ef4515a01a825548a`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate experiment SHA-256:
  `b88ea3ff9b684f0f1780d5286fabfa441071bf86acbe79a00398400d68a4d2a1`.
- Hardware-dot HSACO SHA-256:
  `cc069d0033edc20eb63ff5bc058d4a4e71a5a91e49bcd885a7d97ce4dbf9da21`.
- Scalar HSACO SHA-256:
  `3c593699917d30c33312e89d0d4d0dd92f50221077cbcab6210be3ae72c98e89`.
- Candidate benchmark md5: `c4a3eb86f27b00a88a32a7e32361f64c`.
- Candidate daemon md5: `dbc7a5fdf00c9038992933f29696150b`.
- Candidate DFlash md5: `fe1ee567a4b1ab04363090f8bb15696d`.
- Environment: GPU0-only visibility and the same ROCm 7.14 library,
  model-directory, W7900 baseline, and `scripts/gpu-lock.sh` serialization
  used by the preceding M3 runs.
- Experiment command: three fresh invocations of
  `target/release/examples/bench_mq8_dot_variants`, each with the default
  five-second DPM warm-up. Reports:
  `/tmp/mq8-dot-negative-final-run1.txt`,
  `/tmp/mq8-dot-negative-final-run2.txt`, and
  `/tmp/mq8-dot-negative-final-run3.txt`.
- Correctness commands: `cargo test --workspace --locked --all-targets`,
  `cargo check --workspace --locked --examples`,
  `./scripts/cleanroom-gate.sh`, `./scripts/quant-parity-gate.sh`,
  `./scripts/coherence-gate.sh`, `./scripts/coherence-gate-dflash.sh`, and
  `./scripts/agentic-gate.sh --fast`.
- Performance command: three fresh invocations of
  `./scripts/speed-gate.sh --fast`; each observation is best-of-two.

### Direct MQ8 experiment

| Shape | Dot4 median us | Scalar median us | Scalar/dot4 median ratio | Numerical result |
|---|---:|---:|---:|---|
| KV projection, M=512 K=4096 | 5.729 | 7.101 | 1.242 | bit-exact |
| Square projection, M=4096 K=4096 | 12.944 | 20.493 | 1.573 | bit-exact |
| Gate/up projection, M=11008 K=4096 | 25.427 | 45.519 | 1.791 | bit-exact |
| Down projection, M=4096 K=11008 | 29.182 | 49.457 | 1.704 | bit-exact |

The aggregate scalar/control ratio is 1.635, 1.639, and 1.639 in the three
fresh processes, for a median of 1.639. Reported effective traffic rates are
only a controlled repeated-read comparison and must not be interpreted as
physical HBM bandwidth.

The unbundled gfx1100 metadata reports 33 VGPRs and 33 SGPRs for the dot4
control and 51 VGPRs and 32 SGPRs for the scalar candidate. Both use wave32,
zero private segment bytes, zero LDS, and zero spills. Disassembly contains
ten `v_dot4_i32_iu8` instructions in the control and forty `v_mul_lo_u32`
instructions in the scalar variant. This rules out a spill-induced result and
shows the scalar path's instruction expansion and added register pressure.

### Default-path regression measurements

| Metric | Committed floor | Candidate observation 1 | Candidate observation 2 | Candidate observation 3 | Candidate median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1371.5 | 1310.1 | 1375.9 | 1371.5 |
| 4B MQ4 decode tok/s | 140.9 | 139.9 | 139.7 | 139.7 | 139.7 |

### Decision

- Reject the scalar gfx11 fallback. Its aggregate median latency is 63.9%
  higher than the signed hardware-dot control, with bit-exact outputs.
- Candidate default-path deltas are +11.75% prefill and -0.85% decode. All
  observations pass the committed 5% regression tolerance. The production
  inference hashes are unchanged and this commit adds only an opt-in
  benchmark, so no inference speedup is attributed to it.
- Unified GPU0 quant parity passes all ten cells. Report:
  `/tmp/quant-parity-mq8-negative.md`.
- Four available standard coherence cells pass after manual review. Report:
  `/tmp/coherence-mq8-negative.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-mq8-negative.md`.
- The Qwen3.6 27B fast agentic cell emits valid `name='read'` JSON with zero
  warnings. Report: `/tmp/agentic-gate-mq8-negative.md`.
- Decision row 43 is closed as a measured negative experiment: retain the
  generalized signed hardware dot path and do not merge the scalar candidate.

---

## M4: kernel artifact provenance and staged failures — 2026-08-10

### Run identity

- Regression parent: `d3018c695a22d8ddeae3d6a5d184daed574eb86c`.
- Candidate commit: `19a8e6d6d14bff8dec6af4231c936eb9da53f92b`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Candidate benchmark md5: `a9a5fe0b2c8b6e68a477bca6fc60d791`.
- Candidate daemon md5: `eb74cf1ece190c3772ff9e11c60c3bcf`.
- Candidate DFlash md5: `27d3d1b2b969f17e3d327f2aad54778f`.
- Diagnostic example SHA-256:
  `6263c1bf0dd2d002d6d2edc8068875ab665c704b3d9d3a55658824f0b2dbfc2e`.
- Diagnostic HSACO SHA-256:
  `3a2e5fd0502341f91588c764d6da2e7b6d7923e9b628c7ae7c4c7b7fbd771f79`.
- Environment: GPU0-only visibility and the same ROCm 7.14 library,
  model-directory, W7900 baseline, explicit DPM warm-up in the speed gate,
  and `scripts/gpu-lock.sh` serialization used by the preceding runs.
- Correctness commands: `cargo test --workspace --locked --all-targets`,
  `cargo check --workspace --locked --examples`,
  `cargo test -p rdna-compute --locked --lib`,
  `./scripts/cleanroom-gate.sh`, `./scripts/quant-parity-gate.sh`,
  `./scripts/coherence-gate.sh`, `./scripts/coherence-gate-dflash.sh`, and
  `./scripts/agentic-gate.sh --fast`.
- Diagnostic command: two fresh GPU0 invocations of
  `target/release/examples/inspect_kernel_artifact`, first after creating the
  module and then from the validated cache.
- Performance command: five fresh invocations of
  `./scripts/speed-gate.sh --fast`; each observation is best-of-two. Five
  runs were retained because the preceding batch's unusually high median
  differed by more than 5%.

### Diagnostic result

The final committed example reports:

```text
module=diagnostic_gemv
arch=gfx1100
source_arch_hash=6e0b453068533574
origin=ValidatedCache
validated=true
artifact=.hipfire_kernels/diagnostic_gemv.hsaco
```

The fresh-cache invocation reported `RuntimeCompiled` with the same module,
architecture, combined hash, and artifact path. Unit tests additionally cover
validated precompiled and unvalidated packaged-fallback records. The staged
error test verifies `compile`, `module_load`, and `function_lookup` context,
including preservation of the original HIP error code and explicit
`artifact=unavailable` before a code object exists.

### Default-path regression measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Observation 4 | Observation 5 | Median |
|---|---:|---:|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1276.5 | 1248.9 | 1248.1 | 1252.1 | 1275.7 | 1252.1 |
| 4B MQ4 decode tok/s | 140.9 | 141.1 | 140.6 | 140.1 | 140.1 | 140.3 | 140.3 |

The commit-hook repetition passes at 1369.9 prefill tok/s and 140.8 decode
tok/s.

### Decision

- Candidate median deltas are +2.02% prefill and -0.43% decode. Every
  observation passes the committed 5% regression tolerance. The five-run
  prefill relative spread is 2.27%, so the candidate series itself is stable.
- The change records one small host-side diagnostic per initialized module
  and wraps only initialization failures. The kernel launch path and GPU
  kernels are unchanged, so no speedup is attributed.
- Unified GPU0 quant parity passes all ten cells. Report:
  `/tmp/quant-parity-kernel-diagnostics.md`.
- Four available standard coherence cells pass after manual review. Reports:
  `/tmp/coherence-kernel-diagnostics.md` and hook repetition
  `/tmp/coherence-20260810-110357.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/coherence-dflash-kernel-diagnostics.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero
  warnings. Reports: `/tmp/agentic-gate-kernel-diagnostics.md` and hook
  repetition `/tmp/agentic-gate-20260810-110422.md`.
- Decision row 44 is closed as a diagnostic capability: artifact provenance
  and failure stage can now be queried without changing inference semantics.

---

## M5: clean-room integration composition — 2026-08-10

### Run identity

- Regression parent: `503dfeb5c10c99fd4daf2ae106c184f403359e2f`.
- Candidate implementation commit:
  `0567fb08e6800be837be786702e8ad3c1d9d028d`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Integration-gate SHA-256:
  `34b8b4fe5704351c61a7daf39dbc4e52c57da93210d4f60dd242a190fa9b5934`.
- Plan-test SHA-256:
  `8ff63919dc99dcccd7687814f371e99e7d7a17e0b6e9a99e5bb4983cf1b2e838`.
- Candidate diff SHA-256 recorded by the run:
  `69aad9ff021441d5e3e81c34d6779405d38a28061612bbe84ddf52685b219e2c`.
- Candidate benchmark md5: `a9a5fe0b2c8b6e68a477bca6fc60d791`.
- Candidate daemon md5: `eb74cf1ece190c3772ff9e11c60c3bcf`.
- Rebuilt candidate DFlash md5: `f5f44dfe14f3c142b7716e8b05566d5f`.
- Environment: GPU0-only visibility and the same ROCm 7.14 library,
  model-directory, W7900 baseline, explicit speed-gate DPM warm-up, and
  per-child `scripts/gpu-lock.sh` serialization used by the preceding runs.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row45-46` under the recorded environment.
- Machine manifest:
  `/tmp/hipfire-integration-row45-46/summary.md`.

### Measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1314.0 | 1282.1 | 1262.8 | 1282.1 |
| 4B MQ4 decode tok/s | 140.9 | 141.1 | 140.8 | 140.3 | 140.8 |

### Decision

- Candidate median deltas are +4.47% prefill and -0.07% decode. Every
  observation passes the committed 5% regression tolerance. Relative to M4's
  five-run medians, the differences are +2.40% and +0.36%; neither crosses
  the 5% expansion threshold.
- Full locked workspace all-target tests and example checks pass. The source
  diff, 381-public-function device-binding audit, multi-GPU binding static
  audit, agentic-detector self-check, and clean-room source/license gate pass.
- Unified GPU0 quant parity passes all ten cases. Report:
  `/tmp/hipfire-integration-row45-46/quant-parity.md`.
- Four locally available standard coherence cells pass machine checks and
  manual review. Short-mode truncations are visible and not misclassified as
  semantic corruption. Report:
  `/tmp/hipfire-integration-row45-46/coherence.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row45-46/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Report:
  `/tmp/hipfire-integration-row45-46/agentic.md`.
- PFlash is explicitly skipped because its required local model pair is
  absent and is not presented as passing. Runtime multi-GPU execution is not
  included in this GPU0-only batch; its binding static audit is included.
- Decision: accept rows 45-46 as an independently specified integration
  contract. The scripts-only candidate composes all active GPU0 regression
  gates into one reproducible manifest and does not change inference code.

---

## M6: checked multimodal image input — 2026-08-10

### Run identity

- Regression parent: `575854bdfaf1ef077fdd50f615fb9ddb02b2800e`.
- Candidate commit: `c0ba0b7930da81957bff5980f4d66bbc7162eefe`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Image-boundary SHA-256:
  `72f505ee66360ae2bcc984eaec7c42606a230ef045be22226108510c0fac8b3d`.
- Image-contract test SHA-256:
  `76ab4cdb30e7720240beadf937bb58ea006e4eb070c2836df437b7bb8467226d`.
- Candidate diff SHA-256 recorded by the integration run:
  `910564c847ac3cd960a72ee02b7f863209672fb279d03bc5999260745f965b30`.
- Candidate benchmark md5: `a9a5fe0b2c8b6e68a477bca6fc60d791`.
- Candidate daemon md5: `adeed8e51f6cfca76fc9766aa7abca89`.
- Candidate DFlash md5: `f5f44dfe14f3c142b7716e8b05566d5f`.
- Environment: GPU0-only visibility and the same ROCm 7.14 library,
  model-directory, W7900 baseline, explicit speed-gate DPM warm-up, and
  per-child GPU-lock serialization used by the preceding runs.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row47` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row47/summary.md`.

### Measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1310.7 | 1275.8 | 1268.6 | 1275.8 |
| 4B MQ4 decode tok/s | 140.9 | 141.6 | 141.0 | 140.7 | 141.0 |

The commit-hook repetition passes at 1289.3 prefill tok/s and 140.9 decode
tok/s.

### Decision

- Candidate median deltas are +3.95% prefill and +0.07% decode. Every
  observation passes the committed 5% regression tolerance. Relative to the
  M5 medians, differences are -0.49% and +0.14%; no expansion runs were
  required. The benchmark does not exercise the opt-in vision path, so this
  establishes default-path non-regression and no speedup is attributed.
- Four new image-contract tests and four existing channel-order tests pass.
  Path-backed and decoded inputs produce identical `PreparedImage` values;
  invalid geometry, arithmetic/layout errors, partial patches, and unsafe
  aspect ratios fail before allocation or GPU dispatch.
- Full locked workspace all-target tests, workspace examples, device-binding
  audits, agentic detector self-check, and clean-room source/license gate pass.
- Unified GPU0 quant parity passes all ten cases. Report:
  `/tmp/hipfire-integration-row47/quant-parity.md`.
- Four available standard coherence cells pass after manual review. Reports:
  `/tmp/hipfire-integration-row47/coherence.md` and hook repetition
  `/tmp/coherence-20260810-113428.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row47/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Reports:
  `/tmp/hipfire-integration-row47/agentic.md` and hook repetition
  `/tmp/agentic-gate-20260810-113444.md`.
- No installed HFQ model contains `vision_config`; end-to-end visual encoder
  execution is therefore explicitly unverified on this host rather than
  presented as a pass.
- Decision: accept row 47 as a checked multimodal-input boundary shared by all
  current VL frontends, with the model-dependent GPU vision smoke still open
  until a compatible VL model is available.

---

## M7: unified generation-stop semantics — 2026-08-10

### Run identity

- Regression parent: `4a908c68329b0943604e989ff6b00a76b2ee8a91`.
- Candidate commit: `9c794f3743d2fb78b1f127e54dcff8bcb57b9b5d`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Tokenizer contract SHA-256:
  `c5467624cfd040f0256dfdc26c4376679a89f08e5cedfcc089f246241fa02d22`.
- Generation-semantics audit SHA-256:
  `899508287d3d91ba84c26fe7e53f3029354ca914c4668d3970b81e8896c61eaa`.
- Candidate commit diff SHA-256:
  `89e3ae50cae822fa13a23f4b7348bc2a14a0f8ae57ee2d2d4169056feea387a2`.
- Candidate benchmark md5: `a9a5fe0b2c8b6e68a477bca6fc60d791`.
- Candidate daemon md5: `b55207eb8f080a9c213d700633b573ae`.
- Candidate DFlash md5: `f5f44dfe14f3c142b7716e8b05566d5f`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row48` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row48/summary.md`.

### Measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1300.3 | 1302.2 | 1282.5 | 1300.3 |
| 4B MQ4 decode tok/s | 140.9 | 140.9 | 140.5 | 140.3 | 140.5 |

The commit-hook repetition passes at 1313.8 prefill tok/s and 141.5 decode
tok/s.

### Decision

- Candidate median deltas are +5.95% prefill and -0.28% decode versus the
  committed floor. Every observation passes the 5% regression tolerance.
  Relative to M6 medians the changes are +1.92% and -0.35%, so no two-run
  expansion was required. The inlined stop union is outside GPU compute hot
  paths; no performance improvement is attributed.
- Two generation-stop unit tests pass and cover model EOS, tokenizer EOS,
  auxiliary EOT, active frame stop, duplicate identifiers, absent frame stop,
  and an unrelated token. The static audit passes for all seven user-facing
  decode entry points and is included in the integration manifest.
- Full locked workspace all-target tests, workspace examples, device-binding
  audits, agentic detector self-check, and clean-room source/license gate pass.
- Unified GPU0 quant parity passes all ten cases. Report:
  `/tmp/hipfire-integration-row48/quant-parity.md`.
- Four available standard coherence cells pass after manual review. The 9B
  reasoning sample ends at the short-mode generation bound; all emitted text
  is on-topic and no terminator attracts further output. Reports:
  `/tmp/hipfire-integration-row48/coherence.md` and hook repetition
  `/tmp/coherence-20260810-114200.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Reports:
  `/tmp/hipfire-integration-row48/coherence-dflash.md` and hook repetition
  `/tmp/coherence-dflash-20260810-114224.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Reports:
  `/tmp/hipfire-integration-row48/agentic.md` and hook repetition
  `/tmp/agentic-gate-20260810-114247.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 48 as a single generation-stop contract shared by all
  current user-facing decode paths, with persistent static coverage against
  future entry-point drift.

---

## M8: adapter-owned architecture families — 2026-08-10

### Run identity

- Regression parent: `2ed3959551dc0cc16d56565cdcf18b125be058da`.
- Candidate commit: `22848b873e4fe14264d568241a11fce4d0c36404`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Runtime adapter contract SHA-256:
  `aded4e881760358ff7f6e8ff94af6507d4b8890ae621f92962da3989b15df630`.
- Architecture-adapter audit SHA-256:
  `58f02b4e169ceb835d4cb8babdd64b2cc11c7e3e941a5181cad5105fbcad40ee`.
- Candidate commit diff SHA-256:
  `61744ce082aca3e7f6c326292c6af87a1241d091bf3a6feb25f059af051ecc56`.
- Candidate benchmark md5: `a9a5fe0b2c8b6e68a477bca6fc60d791`.
- Candidate daemon md5: `7ef8596c93667307734125ec4b36902d`.
- Candidate DFlash md5: `f5f44dfe14f3c142b7716e8b05566d5f`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row49` under the recorded environment,
  followed by two fresh `./scripts/speed-gate.sh --fast` processes after the
  cross-batch expansion rule fired.
- Machine manifest: `/tmp/hipfire-integration-row49/summary.md`.

### Measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Observation 4 | Observation 5 | Median |
|---|---:|---:|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1314.2 | 1376.2 | 1378.3 | 1226.7 | 1314.5 | 1314.5 |
| 4B MQ4 decode tok/s | 140.9 | 141.4 | 141.4 | 140.5 | 140.7 | 140.2 | 140.7 |

The commit-hook repetition passes at 1251.4 prefill tok/s and 141.4 decode
tok/s.

### Decision

- The initial three-process prefill median was +5.84% versus M7, so the
  predefined cross-batch rule expanded the sample to five processes. The
  five-process median changes are +1.09% prefill and +0.14% decode versus M7,
  and +7.11% prefill and -0.14% decode versus the committed floor. Every
  observation passes the 5% regression tolerance. Adapter family resolution
  remains outside GPU execution hot paths, so no speedup is attributed.
- Adapter tests prove that LLaMA owns identifiers 0/1, Qwen3.5 and Qwen3.5-VL
  own 5/6, and Toy retains its canonical identifier. Daemon tests prove that
  unsupported identifier 255 fails closed and that protocol labels remain
  stable. The static architecture-adapter audit is included in the integration
  manifest.
- Full locked workspace all-target tests, workspace examples, device-binding
  audits, generation-semantics audit, agentic detector self-check, and
  clean-room source/license gate pass.
- Unified GPU0 quant parity passes all ten cases. Report:
  `/tmp/hipfire-integration-row49/quant-parity.md`.
- Four available standard coherence cells pass after manual review. The 9B
  reasoning sample reaches the short-mode generation bound while remaining
  on-topic; the other outputs give the requested answer or valid tool call.
  Reports: `/tmp/hipfire-integration-row49/coherence.md` and hook repetition
  `/tmp/coherence-20260810-115645.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row49/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Reports:
  `/tmp/hipfire-integration-row49/agentic.md` and hook repetition
  `/tmp/agentic-gate-20260810-115709.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 49 as an adapter-owned architecture-family boundary
  with fail-closed unknown identifiers and shared statically dispatched
  execution foundations.

---

## M9: HFQ consumer-shape contract — 2026-08-10

### Run identity

- Regression parent: `bb127f995b053f92724e4c1124a1d4a1637a2ba0`.
- Candidate commit: `dc2a8ed0aa1ab80cb55884b13823ae38e9e2e4a6`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- HFQ loader SHA-256:
  `28a5651a53d07156bc30c47bee821e0f52c5b12c9386a0aead011bc02d09a6a1`.
- Consumer-shape audit SHA-256:
  `7bd02d12d8a7f3efe10bd2c42ec066257f9b36555384b0919a23ea96b2fbb246`.
- Candidate commit diff SHA-256:
  `d5a1a7659aba15a65e5e52921eee8707765bd467c130e4378dfbfc67903fc573`.
- Candidate benchmark md5: `cedc862bd519478e4c1478e55d79d796`.
- Candidate daemon md5: `6267d1dcf1e8d89160c1dbb3b00e2914`.
- Candidate DFlash md5: `eb1eb35810ac8393b0398ece0281eb1a`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row50` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row50/summary.md`.

### Measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1297.1 | 1249.7 | 1278.2 | 1278.2 |
| 4B MQ4 decode tok/s | 140.9 | 141.3 | 140.8 | 140.3 | 140.8 |

The commit-hook repetition passes at 1303.4 prefill tok/s and 141.1 decode
tok/s.

### Decision

- Candidate median deltas are +4.15% prefill and -0.07% decode versus the
  committed floor. Every observation passes the 5% regression tolerance.
  Relative to M8's five-process medians the changes are -2.76% and +0.07%, so
  no two-run expansion was required. Shape validation runs once while loading
  model bytes and is absent from inference and GPU dispatch hot paths; no
  performance improvement is attributed.
- The consumer-shape unit test accepts exact matrix shapes and intentional
  flattened element counts, then rejects both matrix-dimension and element-count
  mismatches. The static audit requires metadata-bearing Qwen raw loads and
  exact matrix checks in LLaMA, Qwen3.5, and DFlash loaders.
- Full locked workspace all-target tests, workspace examples, device-binding
  audits, architecture-adapter audit, generation-semantics audit, agentic
  detector self-check, and clean-room source/license gate pass.
- Unified GPU0 quant parity passes all ten cases. Reports:
  `/tmp/hipfire-integration-row50/quant-parity.md` and hook repetition
  `/tmp/quant-parity-20260810-121214.md`.
- Four available standard coherence cells pass after manual review. The 4B and
  9B samples reach the short-mode generation bound while remaining on-topic;
  the capital answer and tool call are complete. Reports:
  `/tmp/hipfire-integration-row50/coherence.md` and hook repetition
  `/tmp/coherence-20260810-121224.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Reports:
  `/tmp/hipfire-integration-row50/coherence-dflash.md` and hook repetition
  `/tmp/coherence-dflash-20260810-121251.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Reports:
  `/tmp/hipfire-integration-row50/agentic.md` and hook repetition
  `/tmp/agentic-gate-20260810-121315.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 50 as a load-time contract binding validated HFQ payload
  layouts to model-consumer dimensions before any GPU interpretation.

---

## M10: tokenizer special-token scan — 2026-08-10

### Run identity

- Regression parent: `803adec227cb1caf764f421e6422a799246a7506`.
- Candidate commit: `df8c5408cbfd773513e668d9c974f47980a6e8a8`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Tokenizer SHA-256:
  `944893c4039474114e3fba3be2c021415d116199e47207547e87d0d014bb3bf3`.
- Special-scan audit SHA-256:
  `8db6c61e56d6aea89e281de24c3b72e7126084ba46d099fbfbbe4a264d4a3c87`.
- Tokenizer microbenchmark source SHA-256:
  `21d0328da6a150d5893da5aedeb92d4495075aab5c04f378772ef375020eadca`.
- Candidate commit diff SHA-256:
  `cb338c2544938798d86841339275a6661c27ca25bcc583f7163497cea0477181`.
- Candidate GPU benchmark md5: `2f2e25d00800a1a482d67463990cdbb7`.
- Candidate tokenizer benchmark md5: `d89be854ee8fe777b8d6064bf45a5150`.
- Candidate daemon md5: `6267d1dcf1e8d89160c1dbb3b00e2914`.
- Candidate DFlash md5: `eb1eb35810ac8393b0398ece0281eb1a`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row51` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row51/summary.md`.

### Tokenizer microbenchmark

Each process loads `/home/husrcf/.hipfire/models/qwen3.5-4b.mq4`, runs seven
timed samples of 3,000 encodes, and reports the within-process median. The
table's median is then taken across three fresh processes; lower is better.

| Input | Version | Process 1 ns | Process 2 ns | Process 3 ns | Median ns | Delta |
|---|---|---:|---:|---:|---:|---:|
| Plain prompt | parent | 23356.8 | 23277.4 | 23296.3 | 23296.3 | — |
| Plain prompt | candidate | 22449.6 | 22496.5 | 22852.6 | 22496.5 | -3.43% |
| ChatML-framed prompt | parent | 12955.7 | 12897.1 | 14445.2 | 12955.7 | — |
| ChatML-framed prompt | candidate | 10595.8 | 11689.2 | 10880.2 | 10880.2 | -16.02% |

### GPU measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1406.5 | 1298.9 | 1276.0 | 1298.9 |
| 4B MQ4 decode tok/s | 140.9 | 141.7 | 141.6 | 140.6 | 141.6 |

### Decision

- GPU median deltas are +5.84% prefill and +0.50% decode versus the committed
  floor. Every observation passes the 5% regression tolerance. Relative to M9
  medians the changes are +1.62% and +0.57%, so no two-run expansion was
  required.
- The indexed scanner is byte-identical to the prior linear reference over
  ordinary, overlapping, adjacent, incomplete, and Unicode cases. All 187
  runtime tests pass, and the static audit is included in the integration
  manifest.
- The CPU microbenchmark attributes a 3.43% latency reduction to ordinary
  prompt encoding and a 16.02% reduction to ChatML-framed prompt encoding.
- Full locked workspace all-target tests, workspace examples, device-binding,
  architecture, HFQ-shape, generation, tokenizer, agentic-detector, and
  clean-room source/license audits pass.
- Unified GPU0 quant parity passes all ten cases. Report:
  `/tmp/hipfire-integration-row51/quant-parity.md`.
- Four available standard coherence cells pass after manual review. The 9B
  reasoning sample reaches the short-mode bound while remaining on-topic; the
  other outputs provide the requested answer, code, or valid tool call. Report:
  `/tmp/hipfire-integration-row51/coherence.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row51/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Report:
  `/tmp/hipfire-integration-row51/agentic.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 51 as a semantic-preserving tokenizer hot-path
  improvement, with the CPU speedup separately attributed and the full GPU0
  inference path demonstrated non-regressing.

---

## M11: speculative seed-repeat embedding — 2026-08-10

### Run identity

- Regression parent: `6ea19f00f94eb92f917da69c6ced269df90014b5`.
- Candidate commit: `d283ab580696390018b0e67be0b06ab747ad651c`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Seed-repeat kernel SHA-256:
  `a6153828958a75e21165c9a2dbf2af52347ce65900da31d92c33a00200051bad`.
- Exact-parity example SHA-256:
  `e52ae00249ceda7def2e2d1f326bf112d1d6242613ffab8ff46cc02031c7958a`.
- Static audit SHA-256:
  `cd055deda797f1bfad78dc6313719c787b9ae0b57431f2c79b5b36affe148f44`.
- Candidate commit diff SHA-256:
  `f20def845608fa5ff134a3e5e20bbf8b750871b8da3a08b26ff3723c2877c6c6`.
- Candidate parity binary md5: `8706b5c5ee226bba0ce40aba7a621c61`.
- Candidate DFlash binary md5: `05e01a709fd0b06a55a9d04bd4d1dc4b`.
- Final quant report md5: `fa785bac50e22165f5bf77ac3f690c05`.
- Final DFlash report md5: `77a48f3e3fd61ba69a20f3e60b028dac`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row52` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row52/summary.md`.

### DFlash phase measurements

Each row is a fresh process using the same 27B MQ4 target/drafter pair. Phase
times are synchronized per cycle and averaged within each process; the table
reports medians across three processes. Lower phase time and higher throughput
are better.

| Cell | Version | Throughput tok/s | Draft us | Verify us | Total us |
|---|---|---:|---:|---:|---:|
| Prose | parent | 37.22 | 8469.5 | 47227.1 | 58601.4 |
| Prose | candidate | 36.64 | 8469.2 | 47635.3 | 59010.0 |
| Code | parent | 149.42 | 9699.2 | 52198.2 | 65878.5 |
| Code | candidate | 149.30 | 9649.8 | 52274.5 | 65899.0 |

The code cell is behaviorally stable across all processes (`tau=8.917`, 107
accepted tokens, 119 committed tokens), so its draft-phase delta is directly
comparable: -0.51%. Code throughput changes by -0.08% and total cycle time by
+0.03%. Prose acceptance is stochastic; its median draft time changes by less
than 0.01%, and every output-distribution detector passes.

### General GPU measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1263.7 | 1359.0 | 1277.5 | 1277.5 |
| 4B MQ4 decode tok/s | 140.9 | 140.6 | 139.9 | 139.6 | 139.9 |

### Decision

- The Q8 seed-repeat kernel is bit-exact to seven serial reference lookups and
  is exercised by the installed 27B target. Quant parity passes all 11 cases.
- At `B=16`, candidate construction replaces 16 serial embedding launches
  with one launch. The deterministic code draft phase improves by 0.51%; no
  end-to-end metric regresses materially.
- General GPU medians change by +4.09% prefill and -0.71% decode versus the
  committed W7900 floor. Relative to M10 they change by -1.65% and -1.20%, so
  no five-run expansion was required.
- Full locked workspace all-target tests, workspace examples, device-binding,
  architecture, HFQ-shape, tokenizer, speculative-embedding, generation,
  agentic-detector, and clean-room source/license audits pass.
- Four available standard coherence cells pass after manual review. The 9B
  reasoning sample reaches the short-mode bound while remaining on-topic; the
  other outputs provide the requested answer, code, or valid tool call.
  Report: `/tmp/hipfire-integration-row52/coherence.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row52/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Report:
  `/tmp/hipfire-integration-row52/agentic.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 52 as an exact speculative-input batching improvement
  with a measured draft-phase gain and full GPU0 non-regression evidence.

---

## M12: packed KV footprint record — 2026-08-10

### Run identity

- Regression parent: `fba5d804de4a77ec78ffc49a855f3a406ae9d51a`.
- Candidate commit: `d0b76b3e815bc0fb0bd3d6b0347a542f3c1953af`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Runtime source SHA-256:
  `50c2eaa669125e654b00bbfd92164023d45d8d13f774c26bd9120604fe6b8e16`.
- Footprint example SHA-256:
  `dd149b1476371478242e605f642c39219586c1c76a45a2cd0491489d037817d2`.
- Static audit SHA-256:
  `3cc58e8d541303105fd4123ec7215565354d38d7f5d5bdbe4c472dd43b1f3928`.
- Canonical JSON SHA-256:
  `a69f0fd2ff06ff0fb8300a362d8d8671c2515d5384994f7a05db54cb060593f2`.
- Candidate commit diff SHA-256:
  `71cc336e729fe443715069f44e6cbbbb32b2ff9b91ebe09f065fde35950cf316`.
- Footprint example binary md5: `db897d5de8ed4b6ec929d18f6cac7457`.
- Canonical JSON md5: `95d62a1544a57bfcc04f0c8905aec791`.
- Final quant report md5: `00fb7b6a3848812ab46a463fb84f5e0f`.
- Final standard-coherence report md5: `c17a746fe1fba722a62a8ae46b54eac0`.
- Final DFlash report md5: `db2b85a7ba8a07ee4b5acf67c9048fa8`.
- Final agentic report md5: `7fcc5e30c3e61d2802264929a7d0a454`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row53` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row53/summary.md`.

### Canonical context footprint

The deterministic record uses 16 KV-bearing layers, four KV heads,
`head_dim=256`, logical context 65,536, and physical capacity 2,048. Values
include both K and V plus the exact F32-storage rounding used by constructors.

| Format | K bytes/head | V bytes/head | Total bytes | Total MiB | Reduction vs Q8 |
|---|---:|---:|---:|---:|---:|
| Q8 | 272 | 272 | 71,303,168 | 68.0 | — |
| Asym2 | 68 | 272 | 44,564,480 | 42.5 | 37.50% |
| Asym3 | 100 | 272 | 48,758,784 | 46.5 | 31.62% |
| Asym4 | 132 | 272 | 52,953,088 | 50.5 | 25.74% |

### GPU measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1319.6 | 1296.8 | 1287.6 | 1296.8 |
| 4B MQ4 decode tok/s | 140.9 | 141.5 | 141.1 | 140.6 | 141.1 |

### Decision

- The record is generated by the same checked layout used by packed KV
  constructors; it does not duplicate allocation or kernel-stride formulas.
- All four canonical totals, Asym3 component sizes, logical/physical capacity,
  zero-layer rejection, and aggregate overflow are pinned by unit tests. The
  public planner performs no GPU allocation and is outside inference paths.
- GPU medians change by +5.66% prefill and +0.14% decode versus the committed
  W7900 floor. Relative to M11 they change by +1.51% and +0.86%, so no
  five-run expansion was required.
- Full locked workspace all-target tests, workspace examples, device-binding,
  architecture, HFQ-shape, tokenizer, speculative-embedding, KV-footprint,
  generation, agentic-detector, and clean-room source/license audits pass.
- Unified GPU0 quant parity passes all 11 cases. Report:
  `/tmp/hipfire-integration-row53/quant-parity.md`.
- Four available standard coherence cells pass after manual review. The 4B
  code and 9B reasoning samples reach their short-mode bounds while remaining
  on-topic and non-repetitive; the capital answer and tool call are complete.
  Report: `/tmp/hipfire-integration-row53/coherence.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row53/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Report:
  `/tmp/hipfire-integration-row53/agentic.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 53 as a reproducible, allocation-derived context
  capacity record with full GPU0 non-regression evidence.

---

## M13: generic batched target verify — 2026-08-10

### Run identity

- Regression parent: `6f6661b29478fa40a489a7989b83a55447659799`.
- Candidate commit: `e25078d4105867823a5c4419632cc182c5826924`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Speculative source SHA-256:
  `6b6302c78edf20bf0217c16d42c01cf153d0b6c7184feb1fd4616f5e8883054e`.
- Interactive runner SHA-256:
  `659a60c3c03bdee437e559c5a7584eba0292421ab7503dad31d06556052b3746`.
- Static audit SHA-256:
  `9cf5c93371be26532d55f57b2f96ced60635814d47f05ce209757b5f01183267`.
- Candidate commit diff SHA-256:
  `3621206c21e826513e542e98a178a75fa8cf48da543c0dec6880bf34f53382e4`.
- Final quant report md5: `bb7b53ec4b48b84d10f2ece3957494da`.
- Final standard-coherence report md5: `637b685486575a585032c54a92283ac4`.
- Final DFlash report md5: `85fe77d65bced9bbd24c94714332ce0c`.
- Final agentic report md5: `f08b9267845d40cb23d066e48ab4c958`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row54` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row54/summary.md`.

### GPU measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1295.9 | 1295.6 | 1292.1 | 1295.6 |
| 4B MQ4 decode tok/s | 140.9 | 141.4 | 140.7 | 140.3 | 140.7 |

### Decision

- Generic greedy speculative verification now uses one shared batched target
  call per candidate block instead of one target call per candidate. The
  acceptance planner and rollback/commit replay semantics are unchanged.
- Caller-owned verify scratch is allocated once and reused. A zero-extract
  ring avoids allocating hidden-state history that this generic path does not
  consume.
- GPU medians change by +5.57% prefill and -0.14% decode versus the committed
  W7900 floor. Relative to M12 they change by -0.09% and -0.28%, so no
  five-run expansion was required.
- Full locked workspace all-target tests, workspace examples, device-binding,
  architecture, HFQ-shape, tokenizer, speculative-embedding, greedy-batched-
  verify, KV-footprint, generation, agentic-detector, and clean-room
  source/license audits pass.
- Unified GPU0 quant parity passes all 11 cases. Report:
  `/tmp/hipfire-integration-row54/quant-parity.md`.
- Four available standard coherence cells pass after manual review. The 9B
  reasoning sample reaches its short-mode bound while remaining correct and
  non-repetitive; the capital answer, one-line function, and tool call are
  complete. Report: `/tmp/hipfire-integration-row54/coherence.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row54/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Report:
  `/tmp/hipfire-integration-row54/agentic.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 54 as a distribution-preserving reduction of generic
  target verification calls with full GPU0 non-regression evidence.

---

## M14: hybrid-aware daemon KV allocation — 2026-08-10

### Run identity

- Regression parent: `44e50b13024a63c61bf526d52239c9c5b460393a`.
- Candidate commit: `0098b82`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Daemon source SHA-256:
  `4e92aacb005a7b179a068273c9e9798128daabc5db654617dfada9f87308587f`.
- Static audit SHA-256:
  `d1fb0b2f2080fcff9d0952088d9e3994f50ec0dcc8f057b7609b398e8dab476f`.
- Candidate commit diff SHA-256:
  `a32536b5dc1c791ff381404b88c559707e69c7ce5b701e34e071b4a263bb8ed2`.
- Final quant report md5: `6b06e5f7ca0c7bba904102fe759d8f6a`.
- Final standard-coherence report md5: `28b3900868fcc1958b21ff666e185825`.
- Final DFlash report md5: `129ad1f27c0e1e10dd5ab687a6f85328`.
- Final agentic report md5: `5347fb2b3eb7641e1c567a1f7ec98edc`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row55` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row55/summary.md`.

### Canonical hybrid allocation

The allocation-derived comparison uses 64 total layers, 16 FullAttention
layers, four KV heads, `head_dim=256`, and physical capacity 2,048.

| Format | All-layer allocation | Filtered full buffers | Placeholders | Reduction |
|---|---:|---:|---:|---:|
| Q8 | 272.0 MiB | 68.0 MiB | 384 B | ~75% |
| Asym3 | 186.0 MiB | 46.5 MiB | 384 B | ~75% |

### GPU measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1299.1 | 1275.3 | 1302.9 | 1299.1 |
| 4B MQ4 decode tok/s | 140.9 | 140.9 | 140.5 | 140.0 | 140.5 |

### Decision

- The production single-GPU Q8 and default Asym3 paths allocate full K/V
  buffers only for model-declared FullAttention layers. LinearAttention layer
  indices remain present as one-element placeholders.
- KV-bearing buffers retain the same checked layout, capacity, rotation
  parameters, and kernel strides; the change does not alter stored values.
- GPU medians change by +5.85% prefill and -0.28% decode versus the committed
  W7900 floor. Relative to M13 they change by +0.27% and -0.14%, so no
  five-run expansion was required.
- Full locked workspace all-target tests, workspace examples, all static
  audits including hybrid-KV allocation, and clean-room source/license gates
  pass. Unified GPU0 quant parity passes all 11 cases.
- Four available standard coherence cells pass after manual review. The 9B
  reasoning sample reaches its short-mode bound after deriving the correct
  answer; the capital answer, one-line function, and tool call are complete.
  Report: `/tmp/hipfire-integration-row55/coherence.md`.
- All four DFlash/DDTree cells report `ok=true` and `soft_warn=false`; prose
  and code outputs were manually reviewed. Report:
  `/tmp/hipfire-integration-row55/coherence-dflash.md`.
- The Qwen3.6 27B agentic cell emits valid `name='read'` JSON with zero hard
  failures and zero soft warnings. Report:
  `/tmp/hipfire-integration-row55/agentic.md`.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 55 as a production context-capacity reduction with
  unchanged KV numerical representation and full GPU0 non-regression evidence.

---

## M15: checked hybrid KV capacity record — 2026-08-10

### Run identity

- Regression parent: `a2f70789177ca610611cf7119b5c2cd9525548fb`.
- Candidate commit: `8f349858ba69e9bd927b75627a61a387d4a4b45c`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Runtime source SHA-256:
  `b6b5248facad48aefaf582a03b82d9f4573926d8d55fd8b060288d43ba8b0aa9`.
- Capacity example SHA-256:
  `4bed57a4d7d6f271d52dead5d4db03e0d0292de091f14bbba2609ce75f9cf97d`.
- Static audit SHA-256:
  `fdd5ad2ee7c7de8548b3201684e1c9df895573783c2438479aa9e8d37475800b`.
- Deterministic JSON SHA-256:
  `50dbef9253387d5affa816f66c9006d6f4c60f39f9c38c99c8e3104adf96b531`.
- Candidate commit diff SHA-256:
  `60f31d2dfd0df615d5c7ba04812de88cc071088768b207ec7dbc71bf6bb43905`.
- Final quant report md5: `ab2cf18c1b3d9788a442a2ce6db14da0`.
- Final standard-coherence report md5: `9535fc0729c434ddb600b3e9ff10d483`.
- Final DFlash report md5: `b14ce5c61cebffbc148a2baf5071b1a0`.
- Final agentic report md5: `57f0ea1c5d8ee96d95b21db2754f387f`.
- Environment: GPU0-only ROCr/HIP visibility, ROCm 7.14 library path,
  local read-only model directory, W7900 baseline selection, explicit
  speed-gate DPM warm-up, and per-child GPU-lock serialization.
- Full command: `./scripts/cleanroom-integration-gate.sh --speed-runs 3
  --out /tmp/hipfire-integration-row56` under the recorded environment.
- Machine manifest: `/tmp/hipfire-integration-row56/summary.md`.

### Canonical hybrid allocation

| Format | All-layer bytes | Filtered bytes | Placeholders | Saved bytes |
|---|---:|---:|---:|---:|
| Q8 | 285,212,672 | 71,303,552 | 384 | 213,909,120 |
| Asym3 | 195,035,136 | 48,759,168 | 384 | 146,275,968 |

### GPU measurements

| Metric | Committed floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1287.6 | 1284.5 | 1293.9 | 1287.6 |
| 4B MQ4 decode tok/s | 140.9 | 140.7 | 140.6 | 139.8 | 140.6 |

### Decision

- The public GPU-free record derives all figures from the constructor-shared
  checked layout and includes exact non-KV placeholder storage.
- Unit tests pin Q8 and Asym3 values and reject invalid hybrid layer counts;
  the audit separately requires four base formats and two hybrid records.
- GPU medians change by +4.91% prefill and -0.21% decode versus the committed
  W7900 floor. Relative to M14 they change by -0.89% and +0.07%, so no
  five-run expansion was required.
- Full locked workspace tests, examples, static audits, MIT boundary, and all
  available GPU0 gates pass. Standard, DFlash/DDTree, and agentic outputs were
  manually reviewed; all four DFlash/DDTree detectors report `ok=true` and
  `soft_warn=false`, and agentic reports zero hard failures or soft warnings.
- PFlash remains explicitly skipped because the required local target/drafter
  pair is absent; this is not counted as passing coverage.
- Decision: accept row 56 as a reproducible, checked record of the hybrid KV
  capacity reduction with full GPU0 non-regression evidence.

---

## M16: long-context Q8 position reuse — 2026-08-11

### Run identity

- Regression parent: `8f07fcc07bff2e7ff3852a81dd230e691d8d87f8`.
- Candidate commit: `3a0061382891085135b138b6f7fb554b9224bf69`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Plain runtime SHA-256: `38381df4fc1a20fd15e0d6de2815f42179e05fc5f2869bcc6e45fad0639231a2`.
- Qwen3.5 runtime SHA-256: `6ae6867fc56980dd069444d575ec778f48e9cd07d9ea12330d04e19fe9288955`.
- Static audit SHA-256: `bd4bf325b2c97a46bf07e72c5cb6a07fbac7d1a53ea903f8a966aee539ad8058`.
- Candidate diff SHA-256: `6c3640e77a70a0893ba4bd058d8fb6ca9927c63778f7b2fd8faa466e98e54810`.
- Reports md5: quant `14ad19416b7e4440fbd3ac097d94794b`, standard
  `e17796316eed7ea8256b40c0673b5b8f`, DFlash
  `22ba23a1faa5de218b85fd7de927ca6b`, agentic
  `8cdd291842583283db81fb8db6e290ac`.
- Full gate: `/tmp/hipfire-integration-row57/summary.md`.

### Long-context probe

| Item | Result |
|---|---:|
| Q8 prefill length | 15,001 tokens |
| Prefill wall time | 4,519.2 ms |
| Captured HIP graph blobs | 338 |
| Reference attention at seq 15,006 | 929.7 us |
| Flash attention at seq 15,006 | 166.3 us |
| Flash speedup | 5.59x |
| Maximum absolute delta | 5.25e-6 |

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1266.5 | 1285.7 | 1286.1 | 1285.7 |
| 4B MQ4 decode tok/s | 140.9 | 140.4 | 140.1 | 139.9 | 140.1 |

### Decision

- Three long-context fallbacks reuse the previously uploaded device position
  array; no loop-local allocation or H2D position copy remains.
- The specialized >15K GPU0 probe establishes numerical continuity and graph
  capture, while the normal three-run medians change only -0.15%/-0.36% from
  M15 and remain +4.76%/-0.57% versus the committed floor.
- All available full GPU0 gates pass. Manual review found coherent bounded
  output; all DFlash/DDTree detectors are clean and agentic has zero warnings.
- Decision: accept row 57 as a measured context-movement reduction with full
  MIT-boundary and GPU0 non-regression evidence.

---

## M17: machine-readable long-context Q8 record — 2026-08-11

### Run identity

- Regression parent: `f1b1b98a7a881ba9ae0a6fb1847fc392ed8d0f52`.
- Candidate commit: `885f45c`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Benchmark SHA-256: `fec922ce3812cfba555980774f28ca114ae7313d53b06c789dcad41c07df8d02`.
- Record audit SHA-256: `1f71fee9bca4c237ed1180139b8442825f767ba50a66962c865ea295227c8cac`.
- JSON record SHA-256: `8f0c81792cf4dcaddf25c6ec646d968a0771d4eb91dbe3b89853a316fc493c9c`.
- Candidate diff SHA-256: `57053aae5cffee9413d735d51e1b2804ef14f98b9dacfd7d1f4ca1eac51afeb6`.
- Reports md5: quant `e9570e29443e205d2b1cdfddf77af615`, standard
  `f8c867879ebac364dea9611d5528d2d8`, DFlash
  `2b57beeaa913acdeeee7f0ef32e418a0`, agentic
  `64ab60562ec15b862c1b484b9c1c3833`.
- Full gate: `/tmp/hipfire-integration-row58/summary.md`.

### Recorded long-context result

| Field | Value |
|---|---:|
| Prefill tokens | 15,001 |
| Prefill time | 4,574.5 ms |
| Sequence length | 15,006 |
| Reference attention | 921.9 us |
| Flash attention | 168.8 us |
| Flash speedup | 5.46x |
| Maximum absolute delta | 5.72e-6 |

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1319.6 | 1261.4 | 1274.8 | 1274.8 |
| 4B MQ4 decode tok/s | 140.9 | 141.2 | 140.5 | 139.9 | 140.5 |

### Decision

- The benchmark can persist a versioned record derived from the same measured
  values used by its human-readable output; the audit pins all evidence fields.
- Full CPU and GPU0 gates pass. Normal medians change -0.85%/+0.29% from M16
  and remain +3.87%/-0.28% versus the committed floor.
- Generated outputs were manually reviewed; DFlash/DDTree reports four clean
  detectors and agentic reports zero hard failures and zero soft warnings.
- Decision: accept row 58 as reproducible context-path evidence with full
  performance and numerical-continuity records.

---

## M18: capturable Q8 long-context prefill — 2026-08-11

### Run identity

- Regression parent: `6402f2a04fdd9493adefef05517220ffc10d3420`.
- Candidate commit: `84b5d79c12bb7132c9eec70d29440f6655bd0709`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Qwen3.5 runtime SHA-256: `1e604bb90038727c490bc52c6312615da8414c03dd5e2d034d8b710d327a19c4`.
- Benchmark SHA-256: `40e1157842fbe4788211f55620bc86adc30b7310d8e8399bace34a26c42c1e04`.
- Capture audit SHA-256: `c2274ac046e51b9da620da2b29c99aba97653f30d8e0e29815ef400c61ce56c1`.
- JSON record SHA-256: `2de0ab282f3cdb9760d1fcc9d99c7e71738cb31cc25f9af695d234e772f0b017`.
- Candidate diff SHA-256: `0dfa090c2a7ff055ca632cd48043b9a26bb27e0dfed948886487e81a71984d43`.
- Reports md5: quant `0843cf6cf972b43fe92f19925407d520`, standard
  `81dfc3f52030c29a1f599c78150d66c2`, DFlash
  `c4b59a62e5a82f1e893a7899ec1caeb0`, agentic
  `a791dcdbdcf72b79edec009ff8eb5b52`.
- Full gate: `/tmp/hipfire-integration-row59/summary.md`.

### Captured long-context result

| Field | Value |
|---|---:|
| Initial Q8 prefill tokens | 15,001 |
| Initial prefill time | 4,610.4 ms |
| Captured prefill start position | 15,001 |
| Captured prefill tokens | 2 |
| Captured HIP graph blobs | 447 |
| Measured sequence length | 15,006 |
| Reference attention | 926.5 us |
| Flash attention | 168.4 us |
| Flash speedup | 5.50x |
| Maximum absolute delta | 6.44e-6 |

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1295.8 | 1289.9 | 1268.7 | 1289.9 |
| 4B MQ4 decode tok/s | 140.9 | 140.9 | 140.5 | 140.4 | 140.5 |

### Decision

- Q8 prefill beyond 15K now survives explicit graph capture and launch; the
  final hidden-state copy follows the active capture stream.
- Full CPU and GPU0 gates pass. Normal medians change +1.18%/0.00% from M17
  and remain +5.10%/-0.28% versus the committed floor.
- Generated outputs were manually reviewed; standard and DFlash/DDTree output
  is coherent, all speculative detectors are clean, and agentic reports zero
  hard failures and zero soft warnings.
- Decision: accept row 59 as a measured long-context capture extension with
  full MIT-boundary, numerical-continuity, and performance evidence.

---

## M19: fail-closed architecture-family configuration — 2026-08-11

### Run identity

- Regression parent: `32d8e98c7d1fa810cde6eeb71b9b3ad0b6f2c3d8`.
- Candidate commit: `b3ab9197c72f0cc342d741bf9a0f0aa59ea09e13`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Qwen3.5 adapter SHA-256: `5462ce75661ed67f5eb671113844595c64176e938d924ce375b63c20aa33cde4`.
- LLaMA adapter SHA-256: `4b7a9ff66201daa9d8387e6a156c992c9adddcae00b3a668f1996b155bdd31c2`.
- Qwen3.5-VL adapter SHA-256: `b2a4c67fb01402d325456fe28c6c8b6100beef09fa6d12722a6701400f6fd844`.
- Adapter audit SHA-256: `3a458239f9a570f128eca8502342f5476e8db80c4c126ae86dda78561146fa1a`.
- Candidate diff SHA-256: `46eefa8901717237c0812b44d3f9aa8c7c8cfea35e6fe765db3e34a1693adc55`.
- Reports md5: quant `0c77f2b6775483adbaf44e0799c6e189`, standard
  `d6a1b13a6a98649035d9021953e50ddb`, DFlash
  `a2f0627ef61dc1c6910d0a932e356b16`, agentic
  `90d4598968ece5e47221505581cb2171`.
- Full gate: `/tmp/hipfire-integration-row60/summary.md`.

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1309.9 | 1312.2 | 1281.1 | 1309.9 |
| 4B MQ4 decode tok/s | 140.9 | 140.9 | 140.3 | 139.8 | 140.3 |

### Decision

- All three production adapters reject mismatched HFQ families before
  metadata parsing, keeping architecture differences local to the adapter.
- Full CPU and GPU0 gates pass. Normal medians change +1.55%/-0.14% from M18
  and remain +6.73%/-0.43% versus the committed floor.
- Generated outputs were manually reviewed; standard and DFlash/DDTree output
  is coherent, all speculative detectors are clean, and agentic reports zero
  hard failures and zero soft warnings.
- Decision: accept row 60 as a cold-path model-adapter correctness fix with
  full MIT-boundary and performance non-regression evidence.

---

## M20: adapter-owned protocol labels — 2026-08-11

### Run identity

- Regression parent: `5f980c53415a125271348256c012e21cc50e6e5c`.
- Candidate commit: `de39757`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Architecture contract SHA-256: `33fadcde8fc1ae0c7e8feb4ab083c668cc161ccb3396bf34592925a4c15bdc83`.
- Qwen3.5 adapter SHA-256: `f479d4819cfa32abfa828fbd7b1dfb4a636f5b8f84092755eff2e18f66b6634f`.
- LLaMA adapter SHA-256: `15f99d428ffc84f7b9a279b618b1f75189a214de728bf9b3b7d47317cad77711`.
- Daemon SHA-256: `fb527369bb5ac0d056d72f49f5a00be4ccb807cc014a4c11aa1ff28c92711d29`.
- Adapter audit SHA-256: `8970a5247548ee6c1bd01caec22027fd030f58a4d088a9bb9eaf72630fdc8edd`.
- Candidate diff SHA-256: `76c12e389ee310cd2768dfe1354c2f9b399f481272608201ed56150f24c41c45`.
- Reports md5: quant `5fe644ec5e1c1952e46ae150b802ffd8`, standard
  `e99edee1834e538493d3fdabb6366dfc`, DFlash
  `e495b59d4bed88cf4b15ac20417024f5`, agentic
  `1399986b8fec2fa84a5fb0efd74e094e`.
- Full gate: `/tmp/hipfire-integration-row61/summary.md`.

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1301.5 | 1270.5 | 1314.5 | 1301.5 |
| 4B MQ4 decode tok/s | 140.9 | 141.2 | 140.8 | 140.3 | 140.8 |

### Decision

- Protocol variant naming is owned by family adapters and unsupported IDs
  return no label; the daemon preserves all existing wire values.
- Full CPU and GPU0 gates pass. Normal medians change -0.64%/+0.36% from M19
  and remain +6.05%/-0.07% versus the committed floor.
- Generated outputs were manually reviewed; standard and DFlash/DDTree output
  is coherent, all speculative detectors are clean, and agentic reports zero
  hard failures and zero soft warnings.
- Decision: accept row 61 as a model-adapter capability extension with full
  protocol-compatibility, MIT-boundary, and performance evidence.

---

## M21: stop-before-emit speculative generation — 2026-08-11

### Run identity

- Regression parent: `3ecf73cccecb4430d54e6ec5511a49ce6268cd44`.
- Candidate commit: `52b59340c0ace4c2f005d147e9be684480eb6d1b`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Daemon SHA-256: `48a4249d726a612652e579ed49c160bf7f291653cfde666442c017d50421a34e`.
- Stop-order audit SHA-256: `3fe0398723dca9554c46e2dc1c365d362158e23f01aeaf02660ef9f9a4735f03`.
- Candidate diff SHA-256: `aacfcd216b8430c1148c1f821213069428c700a49f5f7dbbde777727dc4c7b8e`.
- Reports md5: quant `ebcc7cd5d9c0410b9e3c7fc9bf04d09f`, standard
  `a6fe19e9ceaf1b204eb98505340908cc`, DFlash
  `d6373bfbcbd1ae931255b94d06b7646e`, agentic
  `df297126ce5cd977e8a73682645f3b88`.
- Full gate: `/tmp/hipfire-integration-row62/summary.md`; two additional
  `speed-gate.sh --fast` processes supplied observations four and five.

### Standard GPU measurements

| Metric | Floor | Obs. 1 | Obs. 2 | Obs. 3 | Obs. 4 | Obs. 5 | Median |
|---|---:|---:|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1394.6 | 1374.9 | 1302.8 | 1286.3 | 1275.3 | 1302.8 |
| 4B MQ4 decode tok/s | 140.9 | 141.0 | 140.8 | 140.5 | 140.1 | 140.2 | 140.5 |

### Decision

- DFlash/DDTree first and batched stop tokens are classified before token
  events or decoded output; a terminating first token skips speculation.
- Full CPU and GPU0 gates pass. The initial >5% cross-batch prefill change
  triggered five runs; final medians change +0.10%/-0.21% from M20 and remain
  +6.15%/-0.28% versus the committed floor.
- Generated outputs were manually reviewed; standard and DFlash/DDTree output
  is coherent, all speculative detectors are clean, and agentic reports zero
  hard failures and zero soft warnings.
- Decision: accept row 62 as a generation-semantics correction with full
  MIT-boundary, speculative-quality, and performance evidence.

---

## M22: adapter-owned PFlash family checks — 2026-08-11

### Run identity

- Regression parent: `23f1a8ac841cd184e3d24b69b0f712c5ab3762df`.
- Candidate commit: `b0d8ebc3f550fb2dc362dd70ab860968aff97cbe`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- PFlash loader SHA-256: `4d0ea703e5de33060cb792efb13d2f36898a18819d8b1339a7b8f0ffa21e4359`.
- PFlash demo SHA-256: `9f2d148a62919e9acfb25779389d752cc1ff5ab81c2594a37640ae5bada4dd3b`.
- Adapter audit SHA-256: `8b0c045771d2f37ca685f7a9e49042c44f69ac19a08b18e72d0e62f0e2b71434`.
- Candidate diff SHA-256: `a8a3027420dd609e3f8ca47d1829e3eb9326585862b22c2a5780f5355a8cdfc1`.
- Reports md5: quant `514825986154fed3b3a80b34490320d9`, standard
  `c1f5ed00a495ac57ee1512d491960ce5`, DFlash
  `b5346b7e102e523837f3e16f7419b291`, agentic
  `66e124ec2521d19ea206084613f8bb0a`.
- Full gate: `/tmp/hipfire-integration-row63/summary.md`.

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1298.8 | 1327.1 | 1362.2 | 1327.1 |
| 4B MQ4 decode tok/s | 140.9 | 140.2 | 139.3 | 139.8 | 139.8 |

### Decision

- Production and demo PFlash loaders delegate Qwen3.5 family ownership to the
  adapter instead of duplicating architecture IDs.
- Full CPU and GPU0 gates pass. Medians change +1.87%/-0.50% from M21 and
  remain +8.13%/-0.78% versus the committed floor.
- A supplemental 27B MQ4 PFlash run preserved all six historical PFlash PASS
  verdicts and improved four historical baseline-mode FAIL verdicts. Its
  absolute timings are not compared with the unavailable 27B MQ3 baseline.
- Generated outputs were manually reviewed; standard and DFlash/DDTree output
  is coherent, all speculative detectors are clean, and agentic reports zero
  hard failures and zero soft warnings.
- Decision: accept row 63 as a cold-path model-adapter ownership correction
  with full MIT-boundary, quality, and performance non-regression evidence.

---

## M23: HFQ BF16 vision-consumer compatibility — 2026-08-11

### Run identity

- Regression parent: `1c3a44e895543a0b8e7d05aa4c3bf4c65e4af277`.
- Candidate commit: `26d9ca3213e692233f3b60be951eca5f19755246`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Vision loader SHA-256: `ee598d9fb01f735cd026afd9317d976c036358421097d44da774e4a11fcde353`.
- BF16 contract audit SHA-256: `0773e1aed367442cab2b9ea6764c54104cc75bd7dd34adb1fbb5e5b1ce17b68f`.
- Integration gate SHA-256: `81a370078147e44f4e3cf00b0a41ce06dae0d41afcc10e68e82872c5d5bd3291`.
- Candidate diff SHA-256: `5e7bf21b8b11bde92e0c919f30b0334f66efc634de1db74cc7eae690753c9d8b`.
- Reports md5: quant `92688590f854aa73f0a9f9b6cc1262ae`, standard
  `6db5d4307e0ff6f5a3317117e08096ff`, DFlash
  `46ee5b08557a6c30cc73127a7824ebdd`, agentic
  `769b8c4c3a05f07c78a3d461cbadcebe`.
- Full gate: `/tmp/hipfire-integration-row64/summary.md`.

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1286.1 | 1285.6 | 1289.6 | 1286.1 |
| 4B MQ4 decode tok/s | 140.9 | 140.9 | 140.3 | 140.2 | 140.3 |

### Decision

- The qt16 producer, HFQ layout registry, and Qwen3.5-VL F32/F16 consumers
  now agree; deterministic tests pin exact BF16 widening and F16 conversion.
- No qt16 VL model is installed, so real-model visual inference remains
  unclaimed. All available workspace and GPU0 regression gates pass.
- Performance medians change -3.09%/+0.36% from M22 and remain
  +4.79%/-0.43% versus the committed floor.
- Generated outputs were manually reviewed; standard and DFlash/DDTree output
  remains bounded and on-topic, all speculative detectors are clean, and
  agentic reports zero hard failures and zero soft warnings.
- Decision: accept row 64 as a cold-path HFQ format-compatibility correction
  with explicit evidence limits, MIT-boundary checks, and performance
  non-regression evidence.

---

## M24: fail-closed PP/DFlash convergence — 2026-08-11

### Run identity

- Regression parent: `729f2b35fafa2a13d5be7a226f9b18000433a21f`.
- Candidate commit: `4239b527d8c7ae3b15f81981467924365dd7c25e`.
- GPU: Radeon Pro W7900 48GB, `gfx1100`, device 0 only.
- ROCm/HIP: `7.14.60850-0000000`.
- Daemon SHA-256: `df95e4206d7bc1f660fa5bbde0c8f1500cf519aad3d03a6ad98b887844850e01`.
- PP/DFlash audit SHA-256: `780dcb64dc2d86c776a44f940208e0fc9e25bacc60c351f4cf3ce4b616e58c97`.
- Integration gate SHA-256: `af15109ad614040171affa6c02ff4d5e612eaf173b2770d79c71d91810c4f2b5`.
- Candidate diff SHA-256: `b8f817f61139151441ebf436af225a78465f8dbd4fc8ba54b265632b442d2f5c`.
- Reports md5: quant `0614f062a8f127fc0151eb394198e16c`, standard
  `b1ea10a00ad87597f4c314469aa28472`, DFlash
  `9fbdac4fcf5b305de8d8f192ad10ecab`, agentic
  `fa43b1f55e1360e5d482944553860ba0`.
- Full gate: `/tmp/hipfire-integration-row65/summary.md`.

### Standard GPU measurements

| Metric | Floor | Observation 1 | Observation 2 | Observation 3 | Median |
|---|---:|---:|---:|---:|---:|
| 4B MQ4 pp32 prefill tok/s | 1227.3 | 1303.1 | 1285.6 | 1293.3 | 1293.3 |
| 4B MQ4 decode tok/s | 140.9 | 141.5 | 140.8 | 140.5 | 140.8 |

### Decision

- Pipeline parallelism and DFlash remain available independently, while the
  unimplemented combined path now fails closed before either model is loaded.
- The live daemon no longer exposes the experimental `HIPFIRE_PP_DFLASH`
  refusal bypass; historical design records remain intact as audit evidence.
- Full CPU and GPU0 gates pass. Medians change +0.56%/+0.36% from M23 and
  remain +5.38%/-0.07% versus the committed floor, so no five-run expansion
  was required.
- Generated outputs were manually reviewed; standard and DFlash/DDTree output
  is on-topic and non-looping, all speculative detectors are clean, and
  agentic reports zero hard failures and zero soft warnings. The standard 9B
  reasoning sample ends at its fixed token cap while remaining coherent.
- Decision: accept row 65 as a control-plane convergence change with explicit
  fail-closed coverage, MIT-boundary checks, and performance non-regression
  evidence.
