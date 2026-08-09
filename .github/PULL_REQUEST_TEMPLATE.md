<!-- SPDX-License-Identifier: MIT -->
## Summary

<one or two sentences>

## Clean-room declaration

- [ ] I used only the MIT baseline, public specifications/documentation,
      black-box observations, and the metadata-only direction table.
- [ ] I did not inspect or reproduce post-boundary source, diffs, commit
      bodies, file paths, symbols, constants, or implementation details.
- [ ] New or modified implementation files carry an MIT SPDX identifier.

## Which crate(s) does this touch?

- [ ] `kernels/` (HIP source)
- [ ] `crates/rdna-compute` (kernel dispatch / RDNA arch routing)
- [ ] `crates/hip-bridge` (HIP/ROCm FFI)
- [ ] `crates/hipfire-runtime` (LM runtime: KV, sampler, guards, framing, paging, spec decode)
- [ ] `crates/hipfire-arch-qwen35`
- [ ] `crates/hipfire-arch-qwen35-vl`
- [ ] `crates/hipfire-arch-llama`
- [ ] `crates/hipfire-arch-toy` (template — touch only when refining the new-arch reference)
- [ ] `crates/hipfire-quantize`
- [ ] examples / daemon
- [ ] docs / CI / scripts

## Test plan

- [ ] `cargo build --release --workspace --features deltanet` clean
- [ ] `cargo test --lib --workspace --features deltanet` passes
- [ ] If kernel/dispatch changed: `./scripts/coherence-gate.sh` clean
- [ ] If perf-relevant: `./scripts/speed-gate.sh` within ±2% of locked baselines

## Performance evidence

- Baseline commit:
- Candidate commit:
- GPU / ROCm:
- Exact command and flags:
- Prompt path and md5:
- Binary md5:
- Fresh-process runs and medians:
- Correctness/coherence report:

## Architecture-trait change?

If this PR changes the `Architecture` trait surface in
`crates/hipfire-runtime/src/arch.rs`, note here. Trait changes ripple
to every arch crate.
