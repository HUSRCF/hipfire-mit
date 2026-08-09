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
