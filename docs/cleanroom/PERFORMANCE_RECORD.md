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
