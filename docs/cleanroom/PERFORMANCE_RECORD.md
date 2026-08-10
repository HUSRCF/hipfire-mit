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
