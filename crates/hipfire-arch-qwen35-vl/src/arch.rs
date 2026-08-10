// SPDX-License-Identifier: MIT
//! `Architecture` trait implementation for the Qwen3.5-VL vision tower.
//!
//! Mirrors PR 8's pattern (`hipfire-arch-qwen35::arch`): trait-routed
//! bring-up triple, direct static calls for forward dispatch.
//!
//! Scope of the trait impl
//! -----------------------
//! Qwen3.5-VL is "Qwen3.5 text decoder + SigLIP-2 ViT vision tower". The
//! text path is owned by `hipfire-arch-qwen35::Qwen35` and is loaded
//! through THAT trait impl. This trait impl covers ONLY the vision-tower
//! bring-up (config + weights). A vl model load in `daemon.rs` therefore
//! runs both:
//!
//!   let text_cfg = <Qwen35   as Architecture>::config_from_hfq(&hfq)?;
//!   let vis_cfg  = <Qwen35Vl as Architecture>::config_from_hfq(&hfq)?;
//!   let text_w   = <Qwen35   as Architecture>::load_weights(&hfq, &text_cfg, gpu)?;
//!   let vis_w    = <Qwen35Vl as Architecture>::load_weights(&hfq, &vis_cfg, gpu)?;
//!
//! Splitting the two arches at the trait level keeps each crate's bring-up
//! independently exercisable (e.g. unit-testing config parsing for the
//! vision tower without instantiating the full text decoder).
//!
//! Forward calls (`vision_forward`, `forward_scratch`, `forward_scratch_embed`)
//! still go straight to the concrete `pub fn`s in `qwen35_vl` and `qwen35`.
//! See parent crate's `arch.rs` doc for why.
//!
//! `Self::State`
//! -------------
//! The vision tower is stateless one-shot: encode N image patches → N/4
//! visual tokens, splice into the prompt as `<|image_pad|>` substitutions,
//! free the GPU activations. There is no per-step recurrent state to carry
//! across the generation loop, so `State = ()` and `new_state` returns the
//! unit value. (The text decoder's KV / DeltaNet state is owned by the
//! `Qwen35` arch impl, not duplicated here.)

use crate::qwen35_vl::{load_vision_weights, vision_config_from_hfq, VisionConfig, VisionWeights};
use hipfire_runtime::arch::Architecture;
use hipfire_runtime::format::ArchitectureId;
use hipfire_runtime::hfq::HfqFile;
use rdna_compute::Gpu;

/// Type marker for Qwen3.5-VL (vision-language). Loads the SigLIP-2 ViT
/// vision tower; pair with `hipfire-arch-qwen35::Qwen35` for the text
/// decoder side.
pub struct Qwen35Vl;

const VISION_PATCH_EMBED: &str = "model.visual.patch_embed.proj.weight";

fn require_component_config<T>(
    component_present: bool,
    parse: impl FnOnce() -> Result<T, String>,
) -> Result<Option<T>, String> {
    if component_present {
        parse().map(Some)
    } else {
        Ok(None)
    }
}

impl Qwen35Vl {
    /// Parse the optional vision-tower configuration when vision weights are
    /// present. Some text-only exports retain a dormant `vision_config`, so
    /// metadata alone does not activate the adapter. Conversely, an actual
    /// vision tensor bundle must never be silently downgraded to text-only
    /// when its configuration is malformed.
    pub fn config_from_hfq_if_present(hfq: &HfqFile) -> Result<Option<VisionConfig>, String> {
        let has_vision_weights = hfq.tensor_data(VISION_PATCH_EMBED).is_some();
        require_component_config(has_vision_weights, || {
            <Self as Architecture>::config_from_hfq(hfq).map_err(|error| {
                format!(
                    "qwen35-vl: vision weights are present but their configuration is invalid: {error}"
                )
            })
        })
    }
}

impl Architecture for Qwen35Vl {
    type Weights = VisionWeights;
    type State = ();
    type Config = VisionConfig;

    fn arch_id() -> u32 {
        // Qwen3.5-VL ships under arch_id 5/6 (the dense / MoE Qwen3.5
        // identifiers — the VL model is Qwen3.5 dense + ViT). This trait
        // impl returns the same canonical id as Qwen35: 5. The actual id
        // in the HFQ file is the source of truth for the daemon's
        // arch-dispatch ladder.
        ArchitectureId::Qwen35Dense.as_u32()
    }

    fn supports_arch_id(arch_id: u32) -> bool {
        matches!(
            ArchitectureId::from_u32(arch_id),
            Some(ArchitectureId::Qwen35Dense | ArchitectureId::Qwen35Moe)
        )
    }

    fn name() -> &'static str {
        "qwen35-vl"
    }

    fn config_from_hfq(hfq: &HfqFile) -> Result<Self::Config, String> {
        if !Self::supports_arch_id(hfq.arch_id) {
            return Err(format!(
                "qwen35-vl: unsupported HFQ arch_id={} (expected 5 or 6)",
                hfq.arch_id,
            ));
        }
        vision_config_from_hfq(hfq)
            .ok_or_else(|| "qwen35-vl: vision_config not found in HFQ metadata".to_string())
    }

    fn load_weights(
        hfq: &mut HfqFile,
        cfg: &Self::Config,
        gpu: &mut Gpu,
    ) -> Result<Self::Weights, String> {
        load_vision_weights(hfq, cfg, gpu)
            .map_err(|e| format!("qwen35-vl: load_vision_weights failed: {e:?}"))
    }

    fn new_state(_gpu: &mut Gpu, _cfg: &Self::Config) -> Result<Self::State, String> {
        // Vision tower is stateless one-shot — encode patches, emit visual
        // tokens, done. No per-decode-step state to carry. `()` keeps the
        // trait contract uniform without forcing a phantom struct.
        Ok(())
    }

    // No optional overrides needed. VL models reuse Qwen3.5's loop-guard /
    // sampler / prompt-frame / eos-filter conventions through the Qwen35
    // text-side trait impl. The vl impl only owns the vision tower bring-up.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vision_adapter_tracks_the_text_family_ids() {
        assert!(Qwen35Vl::supports_arch_id(5));
        assert!(Qwen35Vl::supports_arch_id(6));
        assert!(!Qwen35Vl::supports_arch_id(1));
        assert!(!Qwen35Vl::supports_arch_id(0xFF));
    }

    #[test]
    fn optional_component_admission_fails_closed_only_when_weights_exist() {
        let absent = require_component_config::<u32>(false, || {
            panic!("text-only models must not parse dormant vision metadata")
        });
        assert_eq!(absent, Ok(None));

        let present = require_component_config(true, || Ok(17u32));
        assert_eq!(present, Ok(Some(17)));

        let malformed = require_component_config::<u32>(true, || Err("bad config".into()));
        assert_eq!(malformed, Err("bad config".into()));
    }
}
