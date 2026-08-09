<!-- SPDX-License-Identifier: MIT -->
# Clean-room implementation policy

This repository continues the verifiable MIT snapshot identified in
`CLEANROOM_BOUNDARY.md`. Its goal is behavioral and performance
compatibility through independent implementation, not source compatibility
with any later-licensed tree.

## Permitted inputs

- The file tree at the tagged MIT baseline.
- Public model specifications, standards, papers, and AMD hardware
  documentation.
- Black-box inputs, outputs, file-format observations, and measurements
  produced from lawfully available binaries or models.
- The hashes, dates, authors, parent counts, high-level categories, and
  abstract principles already recorded in
  `CLEANROOM_COMMIT_DIRECTIONS.md`.

## Prohibited inputs

- Source code, patches, diffs, commit bodies, or unpushed references after
  the boundary.
- Post-boundary file paths, symbols, constants, layouts, thresholds,
  scheduling conditions, pseudocode, or patch summaries.
- Copying code under a license incompatible with an MIT-only outbound
  repository.

Public third-party projects may inform an independent design only when their
license permits the intended use and attribution obligations are recorded.
When uncertain, treat the material as conceptual background and write a new
implementation from the public behavior or specification.

## Required change record

Every implementation change after the baseline must state:

1. The direction-table row or independently stated requirement it addresses.
2. The permitted source of the requirement.
3. The independent design and its acceptance tests.
4. Correctness evidence.
5. Performance evidence, or an explicit statement that the change is not on
   a performance-sensitive path.

Run `./scripts/cleanroom-gate.sh` before committing. Pull requests must
complete the clean-room declaration and performance evidence in the PR
template.

## Performance is a release constraint

Correctness comes first, but a clean-room feature is not complete when it
silently loses material throughput. Performance-sensitive changes use the
committed benchmark prompts and the protocol in
`docs/methodology/perf-benchmarking.md`:

- byte-identical prompts with the prompt md5 recorded;
- a clean candidate binary with its md5 recorded;
- at least three fresh-process samples after warm-up;
- baseline and candidate medians on the same GPU and environment;
- investigation for a median delta whose magnitude is at least 5%;
- the appropriate coherence gate before any speedup claim.

Model files under `~/.hipfire/models/` are read-only test inputs and must
never be copied into this repository.
