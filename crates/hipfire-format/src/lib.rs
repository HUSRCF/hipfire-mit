// SPDX-License-Identifier: MIT
//! Pure, dependency-free HFQ wire-format architecture registry.
//!
//! This crate is shared by model producers, runtime parsers, and architecture
//! adapters. It owns only stable on-disk identifiers and source-model aliases;
//! GPU execution remains in the architecture crates.

/// Stable architecture identifier stored at HFQ header offset `0x08`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ArchitectureId {
    Llama = 0,
    Qwen = 1,
    Qwen35Dense = 5,
    Qwen35Moe = 6,
    DFlashDraft = 20,
}

impl ArchitectureId {
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Llama),
            1 => Some(Self::Qwen),
            5 => Some(Self::Qwen35Dense),
            6 => Some(Self::Qwen35Moe),
            20 => Some(Self::DFlashDraft),
            _ => None,
        }
    }

    /// Stable protocol label for registered wire identifiers.
    pub const fn protocol_label(self) -> &'static str {
        match self {
            Self::Llama => "llama",
            Self::Qwen => "qwen3",
            Self::Qwen35Dense => "qwen3_5",
            Self::Qwen35Moe => "qwen3_5_moe",
            Self::DFlashDraft => "dflash",
        }
    }

    /// Dense/MoE classification for target-model identifiers. Draft formats
    /// return `None` because they are not target execution variants.
    pub const fn target_is_moe(self) -> Option<bool> {
        match self {
            Self::Llama | Self::Qwen | Self::Qwen35Dense => Some(false),
            Self::Qwen35Moe => Some(true),
            Self::DFlashDraft => None,
        }
    }
}

/// Execution family selected by a source model type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    Llama,
    Qwen,
    Qwen35,
}

/// Architecture properties shared by HFQ producers and consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelArchitecture {
    pub id: ArchitectureId,
    pub family: ModelFamily,
    pub is_moe: bool,
}

impl ModelArchitecture {
    /// Resolve public GGUF/Hugging Face model-type aliases supported by the
    /// production target adapters. Unknown names deliberately return `None`.
    pub fn from_model_type(model_type: &str) -> Option<Self> {
        match model_type {
            "llama" | "mistral" => Some(Self {
                id: ArchitectureId::Llama,
                family: ModelFamily::Llama,
                is_moe: false,
            }),
            "qwen2" | "qwen3" => Some(Self {
                id: ArchitectureId::Qwen,
                family: ModelFamily::Qwen,
                is_moe: false,
            }),
            "qwen3_5" | "qwen3_5_text" => Some(Self {
                id: ArchitectureId::Qwen35Dense,
                family: ModelFamily::Qwen35,
                is_moe: false,
            }),
            "qwen3moe" | "qwen3_5_moe" | "qwen3_5_moe_text" => Some(Self {
                id: ArchitectureId::Qwen35Moe,
                family: ModelFamily::Qwen35,
                is_moe: true,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_ids_round_trip_and_keep_protocol_labels() {
        let cases = [
            (ArchitectureId::Llama, "llama"),
            (ArchitectureId::Qwen, "qwen3"),
            (ArchitectureId::Qwen35Dense, "qwen3_5"),
            (ArchitectureId::Qwen35Moe, "qwen3_5_moe"),
            (ArchitectureId::DFlashDraft, "dflash"),
        ];
        for (id, label) in cases {
            assert_eq!(ArchitectureId::from_u32(id.as_u32()), Some(id));
            assert_eq!(id.protocol_label(), label);
        }
        assert_eq!(ArchitectureId::from_u32(0xFF), None);
    }

    #[test]
    fn target_aliases_resolve_to_owned_variants() {
        assert_eq!(
            ModelArchitecture::from_model_type("mistral").unwrap().id,
            ArchitectureId::Llama,
        );
        assert_eq!(
            ModelArchitecture::from_model_type("qwen2").unwrap().id,
            ArchitectureId::Qwen,
        );
        let moe = ModelArchitecture::from_model_type("qwen3_5_moe_text").unwrap();
        assert_eq!(moe.id, ArchitectureId::Qwen35Moe);
        assert!(moe.is_moe);
        assert_eq!(ModelArchitecture::from_model_type("gemma4"), None);
        assert_eq!(ModelArchitecture::from_model_type("dflash"), None);
    }

    #[test]
    fn only_target_ids_have_dense_moe_classification() {
        assert_eq!(ArchitectureId::Llama.target_is_moe(), Some(false));
        assert_eq!(ArchitectureId::Qwen35Dense.target_is_moe(), Some(false));
        assert_eq!(ArchitectureId::Qwen35Moe.target_is_moe(), Some(true));
        assert_eq!(ArchitectureId::DFlashDraft.target_is_moe(), None);
    }
}
