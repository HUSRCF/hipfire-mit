// SPDX-License-Identifier: MIT
//! `Architecture` trait implementation for Qwen3.5.
//!
//! This is the canary arch implementation (PR 8 of
//! `docs/plans/engine-modularization.prd`). The trait surface here defines
//! what later arch crates (qwen35-vl in PR 9, llama in PR 11) must
//! implement.
//!
//! Forward-pass dispatch is INTENTIONALLY NOT routed through the trait.
//! `daemon.rs` and other consumers call `qwen35::forward_scratch`,
//! `qwen35::forward_prefill_batch`, etc. directly. Reasons:
//!   1. Forward signatures vary heavily across arches (number of buffers,
//!      KV layout, hybrid-vs-dense paths, vision conditioning, MoE expert
//!      management). Forcing a single trait shape would either bloat the
//!      contract or hide essential parameters behind opaque slots.
//!   2. Forward dispatch is hot-path. Static dispatch via concrete-type
//!      function calls keeps the call graph fully inlinable; dyn-trait
//!      dispatch in the inner loop costs measurable tok/s on small models.
//!   3. The trait's job is BRING-UP scaffolding (load → instantiate →
//!      generation-loop wiring), not runtime polymorphism. Once an arch
//!      is loaded, the daemon/CLI knows the concrete type at compile time.
//!
//! The trait gives:
//!   - one entry point per arch for config parsing + weight load + state
//!     init (the bring-up triple),
//!   - a place to register arch-specific overrides for loop_guard /
//!     sampler / prompt_frame / eos_filter without growing daemon's
//!     `match arch_id` ladder,
//!   - a discoverable contract for adding a new arch ("implement this trait
//!     and register your `arch_id`").

use crate::qwen35::{
    config_from_hfq as qwen35_config_from_hfq, load_weights as qwen35_load_weights, DeltaNetState,
    Qwen35Config, Qwen35Weights,
};
use hipfire_runtime::arch::Architecture;
use hipfire_runtime::format::ArchitectureId;
use hipfire_runtime::hfq::HfqFile;
use rdna_compute::Gpu;

/// Type marker for Qwen3.5 architecture (dense Qwen3.5 0.8B/4B/9B/27B,
/// MoE Qwen3.5-A3B/A10B/A17B, dense Qwen3.6, MoE Qwen3.6-A3B). All share
/// the hybrid DeltaNet + FullAttention layer scheme.
pub struct Qwen35;

/// Validate the on-disk dense/MoE discriminator against the parsed model
/// shape. Kept beside adapter ownership so config consumers cannot maintain a
/// second, drifting interpretation of architecture ids.
pub(crate) fn variant_matches_config(arch_id: u32, is_moe: bool) -> bool {
    Qwen35::is_moe_arch_id(arch_id) == Some(is_moe)
}

impl Architecture for Qwen35 {
    type Weights = Qwen35Weights;
    type State = DeltaNetState;
    type Config = Qwen35Config;

    fn arch_id() -> u32 {
        // arch_id 5 = Qwen3.5 dense, arch_id 6 = Qwen3.5/3.6 MoE (A3B).
        // Returns the dense ID as the canonical "Qwen3.5 family" marker;
        // the actual id loaded at runtime is on `HfqFile::arch_id` and is
        // either 5 or 6.
        ArchitectureId::Qwen35Dense.as_u32()
    }

    fn supports_arch_id(arch_id: u32) -> bool {
        matches!(
            ArchitectureId::from_u32(arch_id),
            Some(ArchitectureId::Qwen35Dense | ArchitectureId::Qwen35Moe)
        )
    }

    fn name() -> &'static str {
        "qwen35"
    }

    fn protocol_label(arch_id: u32) -> Option<&'static str> {
        match ArchitectureId::from_u32(arch_id) {
            Some(id @ (ArchitectureId::Qwen35Dense | ArchitectureId::Qwen35Moe)) => {
                Some(id.protocol_label())
            }
            _ => None,
        }
    }

    fn is_moe_arch_id(arch_id: u32) -> Option<bool> {
        match ArchitectureId::from_u32(arch_id) {
            Some(id @ (ArchitectureId::Qwen35Dense | ArchitectureId::Qwen35Moe)) => {
                id.target_is_moe()
            }
            _ => None,
        }
    }

    fn config_from_hfq(hfq: &HfqFile) -> Result<Self::Config, String> {
        if !Self::supports_arch_id(hfq.arch_id) {
            return Err(format!(
                "qwen35: unsupported HFQ arch_id={} (expected 5 or 6)",
                hfq.arch_id,
            ));
        }
        qwen35_config_from_hfq(hfq)
            .ok_or_else(|| "qwen35: failed to parse config from HFQ metadata".to_string())
    }

    fn load_weights(
        hfq: &mut HfqFile,
        cfg: &Self::Config,
        gpu: &mut Gpu,
    ) -> Result<Self::Weights, String> {
        qwen35_load_weights(hfq, cfg, gpu)
            .map_err(|e| format!("qwen35: load_weights failed: {e:?}"))
    }

    fn new_state(gpu: &mut Gpu, cfg: &Self::Config) -> Result<Self::State, String> {
        DeltaNetState::new(gpu, cfg)
            .map_err(|e| format!("qwen35: DeltaNetState::new failed: {e:?}"))
    }

    // Optional overrides default to the trait scaffold's Qwen3.5-flavored
    // baseline. Qwen3.5 IS the canonical arch the trait was designed
    // around, so no overrides needed here. Future arches (gemma4, llama)
    // will exercise the override surface.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_owns_dense_and_moe_family_ids() {
        assert!(Qwen35::supports_arch_id(5));
        assert!(Qwen35::supports_arch_id(6));
        assert!(!Qwen35::supports_arch_id(0));
        assert!(!Qwen35::supports_arch_id(0xFF));
        assert_eq!(Qwen35::protocol_label(5), Some("qwen3_5"));
        assert_eq!(Qwen35::protocol_label(6), Some("qwen3_5_moe"));
        assert_eq!(Qwen35::protocol_label(0xFF), None);
        assert_eq!(Qwen35::is_moe_arch_id(5), Some(false));
        assert_eq!(Qwen35::is_moe_arch_id(6), Some(true));
        assert_eq!(Qwen35::is_moe_arch_id(0xFF), None);
    }

    #[test]
    fn adapter_rejects_header_and_config_variant_mismatches() {
        assert!(variant_matches_config(5, false));
        assert!(variant_matches_config(6, true));
        assert!(!variant_matches_config(5, true));
        assert!(!variant_matches_config(6, false));
        assert!(!variant_matches_config(0xFF, false));
    }
}
