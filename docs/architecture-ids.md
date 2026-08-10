<!-- SPDX-License-Identifier: MIT -->
# HFQ architecture identifiers

The canonical machine-readable registry is
`crates/hipfire-format/src/lib.rs`. Producers, runtime parsers, and production
architecture adapters consume that dependency-free crate instead of keeping
parallel numeric maps.

| ID | Registry variant | Protocol label | Role |
|---:|---|---|---|
| 0 | `Llama` | `llama` | LLaMA/Mistral target |
| 1 | `Qwen` | `qwen3` | Qwen2/Qwen3 target |
| 5 | `Qwen35Dense` | `qwen3_5` | dense Qwen3.5/3.6 target |
| 6 | `Qwen35Moe` | `qwen3_5_moe` | MoE Qwen3/Qwen3.5/3.6 target |
| 20 | `DFlashDraft` | `dflash` | DFlash draft format |

`0xFF` remains reserved for the non-production toy adapter and is not a
registered on-disk format. New IDs must be added to the registry with
round-trip, protocol-label, and producer/consumer coverage before use.

Target-model descriptors have no public fields: family and dense/MoE
properties are derived from the registered ID. Draft-only IDs cannot be
constructed as target descriptors, preventing contradictory routing metadata.
