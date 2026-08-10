// SPDX-License-Identifier: MIT
//! HFQ (.hfq) file loader for hipfire-native Q4_F16 quantized models.

use crate::llama::{
    f16_to_f32, EmbeddingFormat, LayerWeights, LlamaConfig, LlamaWeights, ModelArch, WeightTensor,
};
use hip_bridge::{HipError, HipResult};
use memmap2::Mmap;
use rdna_compute::{DType, Gpu, GpuTensor};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::Path;

const HFQ_MAGIC: &[u8; 4] = b"HFQM";
const HFQ_VERSION_SUPPORTED: u32 = 1;
const HFQ_HEADER_SIZE: usize = 32;

/// Drop page cache for a file byte range via posix_fadvise(FADV_DONTNEED).
/// On unified-memory APUs (e.g. Strix Halo), mmap'd model data and
/// hipMalloc'd GPU copies share physical RAM — without this, loading
/// a 65 GB model consumes ~130 GB (mmap cache + GPU copy).
/// Note: madvise(MADV_DONTNEED) does NOT work on MAP_SHARED file-backed
/// mappings (memmap2 default). posix_fadvise on the fd does.
#[cfg(unix)]
fn fadvise_dontneed(fd: std::os::unix::io::RawFd, offset: usize, len: usize) {
    unsafe {
        libc::posix_fadvise(fd, offset as libc::off_t, len as libc::off_t, libc::POSIX_FADV_DONTNEED);
    }
}

#[cfg(not(unix))]
fn fadvise_dontneed(_fd: i32, _offset: usize, _len: usize) {}

pub struct HfqTensorInfo {
    pub name: String,
    /// HFQ wire-format identifier; layout validation below owns the registry.
    pub quant_type: u8,
    pub shape: Vec<u32>,
    pub group_size: u32,
    pub data_offset: usize,
    pub data_size: usize,
}

impl HfqTensorInfo {
    /// Require the file-declared tensor shape to match the model consumer.
    ///
    /// Index validation proves that a payload is self-consistent with its own
    /// declaration. This second, cold-path check binds that declaration to the
    /// dimensions derived from model configuration before a GPU kernel can use
    /// `m` and `k` to interpret the bytes.
    pub fn expect_shape(&self, expected: &[usize]) -> HipResult<()> {
        let matches = self.shape.len() == expected.len()
            && self
                .shape
                .iter()
                .zip(expected)
                .all(|(&actual, &wanted)| actual as usize == wanted);
        if matches {
            return Ok(());
        }
        Err(HipError::new(
            0,
            &format!(
                "HFQ tensor {:?} shape mismatch: file {:?}, model {:?}",
                self.name, self.shape, expected
            ),
        ))
    }

    /// Require only the flattened element count when the consumer explicitly
    /// treats a higher-rank tensor as a vector.
    pub fn expect_numel(&self, expected: usize) -> HipResult<()> {
        let actual = self.shape.iter().try_fold(1usize, |numel, &dim| {
            numel.checked_mul(dim as usize)
        });
        if actual == Some(expected) {
            return Ok(());
        }
        Err(HipError::new(
            0,
            &format!(
                "HFQ tensor {:?} element-count mismatch: file {:?} ({actual:?}), model {expected}",
                self.name, self.shape
            ),
        ))
    }
}

pub struct HfqFile {
    _file: File,
    /// Path used to open the file. Exposed via [`Self::path`] so the
    /// weight pager can open its own file handle for paged reads without
    /// going through this struct (cleanly separates HfqFile's mmap-based
    /// tensor lookup from the pager's pread/io_uring transport).
    path: std::path::PathBuf,
    /// mmap for tensor data access on discrete-GPU systems where GPU VRAM
    /// is separate from system RAM (no double-buffering cost).
    /// `None` on unified-memory APUs (Strix Halo etc.) where mmap pages
    /// and hipMalloc share physical RAM — keeping the mmap alive doubles
    /// memory consumption. Dropped after header/index parsing via
    /// `drop_mmap()`. When `None`, all tensor reads go through `pread`.
    mmap: Option<Mmap>,
    pub arch_id: u32,
    pub metadata_json: String,
    tensors: Vec<HfqTensorInfo>,
    tensor_map: HashMap<String, usize>,
    /// Reusable read buffer for pread-based tensor reads.
    /// Avoids page cache buildup on unified-memory APUs where mmap pages
    /// can't be evicted while the mapping exists (FADV_DONTNEED is ignored
    /// for mmap'd regions per Linux kernel docs).
    pread_buf: std::cell::RefCell<Vec<u8>>,
}

struct ParsedHfq {
    arch_id: u32,
    metadata_json: String,
    tensors: Vec<HfqTensorInfo>,
    tensor_map: HashMap<String, usize>,
}

fn invalid_hfq(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message.into())
}

fn take_hfq_bytes<'a>(
    bytes: &'a [u8],
    pos: &mut usize,
    limit: usize,
    len: usize,
    field: &str,
) -> io::Result<&'a [u8]> {
    let end = pos
        .checked_add(len)
        .ok_or_else(|| invalid_hfq(format!("HFQ {field} offset overflow")))?;
    if end > limit || end > bytes.len() {
        return Err(invalid_hfq(format!("truncated HFQ {field}")));
    }
    let out = &bytes[*pos..end];
    *pos = end;
    Ok(out)
}

fn read_hfq_u8(bytes: &[u8], pos: &mut usize, limit: usize, field: &str) -> io::Result<u8> {
    Ok(take_hfq_bytes(bytes, pos, limit, 1, field)?[0])
}

fn read_hfq_u16(bytes: &[u8], pos: &mut usize, limit: usize, field: &str) -> io::Result<u16> {
    let raw = take_hfq_bytes(bytes, pos, limit, 2, field)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_hfq_u32(bytes: &[u8], pos: &mut usize, limit: usize, field: &str) -> io::Result<u32> {
    let raw = take_hfq_bytes(bytes, pos, limit, 4, field)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_hfq_u64(bytes: &[u8], pos: &mut usize, limit: usize, field: &str) -> io::Result<u64> {
    let raw = take_hfq_bytes(bytes, pos, limit, 8, field)?;
    Ok(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

fn hfq_usize(value: u64, field: &str) -> io::Result<usize> {
    usize::try_from(value).map_err(|_| invalid_hfq(format!("HFQ {field} does not fit this host")))
}

fn hfq_numel(name: &str, shape: &[u32]) -> io::Result<usize> {
    shape.iter().try_fold(1usize, |numel, &dim| {
        numel.checked_mul(dim as usize).ok_or_else(|| {
            invalid_hfq(format!("HFQ tensor {name:?} element count overflows this host"))
        })
    })
}

fn hfq_block_payload_size(
    name: &str,
    numel: usize,
    group_elems: usize,
    block_bytes: usize,
) -> io::Result<usize> {
    let blocks = numel / group_elems + usize::from(numel % group_elems != 0);
    blocks.checked_mul(block_bytes).ok_or_else(|| {
        invalid_hfq(format!("HFQ tensor {name:?} payload size overflows this host"))
    })
}

fn validate_hfq_tensor_layout(
    name: &str,
    quant_type: u8,
    shape: &[u32],
    group_size: u32,
    data_size: usize,
) -> io::Result<()> {
    enum Layout {
        Dense { bytes_per_element: usize },
        Blocks { group_elems: usize, block_bytes: usize },
        Q8Hfq,
        Hfp4G32,
    }

    let layout = match quant_type {
        0 => Layout::Blocks { group_elems: 64, block_bytes: 36 },
        1 | 16 => Layout::Dense { bytes_per_element: 2 },
        2 => Layout::Dense { bytes_per_element: 4 },
        3 => Layout::Blocks { group_elems: 32, block_bytes: 34 },
        4 => Layout::Blocks { group_elems: 256, block_bytes: 144 },
        5 => Layout::Q8Hfq,
        6 | 13 => Layout::Blocks { group_elems: 256, block_bytes: 136 },
        7 => Layout::Blocks { group_elems: 128, block_bytes: 72 },
        8 | 15 => Layout::Blocks { group_elems: 256, block_bytes: 200 },
        9 | 18 | 19 => Layout::Blocks { group_elems: 256, block_bytes: 72 },
        10 => Layout::Blocks { group_elems: 128, block_bytes: 40 },
        11 | 17 => Layout::Blocks { group_elems: 256, block_bytes: 104 },
        12 => Layout::Blocks { group_elems: 128, block_bytes: 56 },
        14 => Layout::Blocks { group_elems: 256, block_bytes: 258 },
        20 => Layout::Blocks { group_elems: 256, block_bytes: 112 },
        21 | 24 => Layout::Hfp4G32,
        _ => {
            return Err(invalid_hfq(format!(
                "HFQ tensor {name:?} uses unsupported quant_type {quant_type}"
            )))
        }
    };

    let numel = hfq_numel(name, shape)?;
    let (expected_group_size, expected_data_size) = match layout {
        Layout::Dense { bytes_per_element } => (
            0u32,
            numel.checked_mul(bytes_per_element).ok_or_else(|| {
                invalid_hfq(format!("HFQ tensor {name:?} payload size overflows this host"))
            })?,
        ),
        Layout::Blocks { group_elems, block_bytes } => (
            group_elems as u32,
            hfq_block_payload_size(name, numel, group_elems, block_bytes)?,
        ),
        Layout::Q8Hfq => {
            if shape.len() != 2 {
                return Err(invalid_hfq(format!(
                    "HFQ Q8HFQ tensor {name:?} must be 2D, got shape {shape:?}"
                )));
            }
            let rows = shape[0] as usize;
            let columns = shape[1] as usize;
            if columns % 32 != 0 {
                return Err(invalid_hfq(format!(
                    "HFQ Q8HFQ tensor {name:?} columns ({columns}) are not divisible by 32"
                )));
            }
            let raw_row = columns
                .checked_add((columns / 32).checked_mul(2).ok_or_else(|| {
                    invalid_hfq(format!("HFQ tensor {name:?} row size overflows this host"))
                })?)
                .ok_or_else(|| {
                    invalid_hfq(format!("HFQ tensor {name:?} row size overflows this host"))
                })?;
            let row_stride = raw_row.checked_add(127).ok_or_else(|| {
                invalid_hfq(format!("HFQ tensor {name:?} row alignment overflows this host"))
            })? & !127;
            (
                32,
                rows.checked_mul(row_stride).ok_or_else(|| {
                    invalid_hfq(format!("HFQ tensor {name:?} payload size overflows this host"))
                })?,
            )
        }
        Layout::Hfp4G32 => {
            if shape.len() != 2 {
                return Err(invalid_hfq(format!(
                    "HFQ FP4 tensor {name:?} must be 2D, got shape {shape:?}"
                )));
            }
            let rows = shape[0] as usize;
            let columns = shape[1] as usize;
            if columns % 256 != 0 {
                return Err(invalid_hfq(format!(
                    "HFQ FP4 tensor {name:?} columns ({columns}) are not divisible by 256"
                )));
            }
            let block_bytes = (columns / 32).checked_mul(17).ok_or_else(|| {
                invalid_hfq(format!("HFQ tensor {name:?} row size overflows this host"))
            })?;
            let row_bytes = 16usize.checked_add(block_bytes).ok_or_else(|| {
                invalid_hfq(format!("HFQ tensor {name:?} row size overflows this host"))
            })?;
            (
                32,
                rows.checked_mul(row_bytes).ok_or_else(|| {
                    invalid_hfq(format!("HFQ tensor {name:?} payload size overflows this host"))
                })?,
            )
        }
    };

    if group_size != expected_group_size {
        return Err(invalid_hfq(format!(
            "HFQ tensor {name:?} quant_type {quant_type} has group_size {group_size}, expected {expected_group_size}"
        )));
    }
    if data_size != expected_data_size {
        return Err(invalid_hfq(format!(
            "HFQ tensor {name:?} quant_type {quant_type} payload is {data_size} bytes, expected {expected_data_size} for shape {shape:?}"
        )));
    }
    Ok(())
}

/// Parse and validate the complete HFQ header and tensor index before any
/// input-derived slice is constructed. Payload contents remain opaque, while
/// the quantization type, group size, shape, and declared byte length must
/// agree before a loader can expose the range to a GPU dispatch.
fn parse_hfq_bytes(bytes: &[u8]) -> io::Result<ParsedHfq> {
    if bytes.len() < HFQ_HEADER_SIZE {
        return Err(invalid_hfq(format!(
            "truncated HFQ header: {} bytes (need {HFQ_HEADER_SIZE})",
            bytes.len()
        )));
    }
    if bytes.get(..HFQ_MAGIC.len()) != Some(HFQ_MAGIC.as_slice()) {
        return Err(invalid_hfq("invalid HFQ magic"));
    }

    let mut header_pos = HFQ_MAGIC.len();
    let version = read_hfq_u32(bytes, &mut header_pos, HFQ_HEADER_SIZE, "version")?;
    if version != HFQ_VERSION_SUPPORTED {
        return Err(invalid_hfq(format!(
            "unsupported HFQ version {version}; this build supports {HFQ_VERSION_SUPPORTED}"
        )));
    }
    let arch_id = read_hfq_u32(bytes, &mut header_pos, HFQ_HEADER_SIZE, "architecture")?;
    let n_tensors = read_hfq_u32(bytes, &mut header_pos, HFQ_HEADER_SIZE, "tensor count")? as usize;
    let metadata_offset = hfq_usize(
        read_hfq_u64(bytes, &mut header_pos, HFQ_HEADER_SIZE, "metadata offset")?,
        "metadata offset",
    )?;
    let data_offset = hfq_usize(
        read_hfq_u64(bytes, &mut header_pos, HFQ_HEADER_SIZE, "data offset")?,
        "data offset",
    )?;

    if metadata_offset < HFQ_HEADER_SIZE {
        return Err(invalid_hfq(format!(
            "HFQ metadata offset {metadata_offset} overlaps the {HFQ_HEADER_SIZE}-byte header"
        )));
    }
    if metadata_offset >= data_offset {
        return Err(invalid_hfq(format!(
            "HFQ offsets are not ordered: metadata={metadata_offset}, data={data_offset}"
        )));
    }
    if data_offset > bytes.len() {
        return Err(invalid_hfq(format!(
            "HFQ data offset {data_offset} exceeds file length {}",
            bytes.len()
        )));
    }

    let metadata_and_index = &bytes[metadata_offset..data_offset];
    if metadata_and_index
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        != Some(b'{')
    {
        return Err(invalid_hfq("HFQ metadata JSON must be an object"));
    }
    let mut metadata_stream = serde_json::Deserializer::from_slice(metadata_and_index)
        .into_iter::<serde::de::IgnoredAny>();
    metadata_stream
        .next()
        .ok_or_else(|| invalid_hfq("HFQ metadata JSON is empty"))?
        .map_err(|err| invalid_hfq(format!("invalid HFQ metadata JSON: {err}")))?;
    let metadata_len = metadata_stream.byte_offset();
    if metadata_len == 0 {
        return Err(invalid_hfq("HFQ metadata JSON has no complete value"));
    }
    let metadata_json = std::str::from_utf8(&metadata_and_index[..metadata_len])
        .map_err(|err| invalid_hfq(format!("HFQ metadata is not UTF-8: {err}")))?
        .to_owned();

    let mut pos = metadata_offset
        .checked_add(metadata_len)
        .ok_or_else(|| invalid_hfq("HFQ metadata end offset overflow"))?;
    let index_count = read_hfq_u32(bytes, &mut pos, data_offset, "index tensor count")? as usize;
    if index_count != n_tensors {
        return Err(invalid_hfq(format!(
            "HFQ tensor count mismatch: header={n_tensors}, index={index_count}"
        )));
    }

    // Even a scalar with an empty name needs 16 bytes of fixed index data.
    // Reject impossible counts before reserving attacker-controlled capacity.
    const MIN_TENSOR_RECORD_BYTES: usize = 2 + 1 + 1 + 4 + 8;
    if n_tensors > (data_offset - pos) / MIN_TENSOR_RECORD_BYTES {
        return Err(invalid_hfq(format!(
            "HFQ tensor count {n_tensors} cannot fit in the declared index"
        )));
    }

    let mut tensors = Vec::with_capacity(n_tensors);
    let mut tensor_map = HashMap::with_capacity(n_tensors);
    let mut cumulative_offset = data_offset;

    for tensor_idx in 0..n_tensors {
        let name_len = read_hfq_u16(bytes, &mut pos, data_offset, "tensor name length")? as usize;
        if name_len == 0 {
            return Err(invalid_hfq(format!(
                "HFQ tensor {tensor_idx} has an empty name"
            )));
        }
        let name_bytes = take_hfq_bytes(bytes, &mut pos, data_offset, name_len, "tensor name")?;
        let name = std::str::from_utf8(name_bytes)
            .map_err(|err| {
                invalid_hfq(format!("HFQ tensor {tensor_idx} name is not UTF-8: {err}"))
            })?
            .to_owned();
        let quant_type = read_hfq_u8(bytes, &mut pos, data_offset, "tensor quant type")?;
        let n_dims = read_hfq_u8(bytes, &mut pos, data_offset, "tensor dimension count")? as usize;
        let mut shape = Vec::with_capacity(n_dims);
        for _ in 0..n_dims {
            shape.push(read_hfq_u32(
                bytes,
                &mut pos,
                data_offset,
                "tensor dimension",
            )?);
        }
        let group_size = read_hfq_u32(bytes, &mut pos, data_offset, "tensor group size")?;
        let data_size = hfq_usize(
            read_hfq_u64(bytes, &mut pos, data_offset, "tensor data size")?,
            "tensor data size",
        )?;
        validate_hfq_tensor_layout(&name, quant_type, &shape, group_size, data_size)?;
        let tensor_end = cumulative_offset
            .checked_add(data_size)
            .ok_or_else(|| invalid_hfq(format!("HFQ tensor {name:?} data range overflows")))?;
        if tensor_end > bytes.len() {
            return Err(invalid_hfq(format!(
                "HFQ tensor {name:?} data range {cumulative_offset}..{tensor_end} exceeds file length {}",
                bytes.len()
            )));
        }
        if tensor_map.insert(name.clone(), tensor_idx).is_some() {
            return Err(invalid_hfq(format!("duplicate HFQ tensor name {name:?}")));
        }

        tensors.push(HfqTensorInfo {
            name,
            quant_type,
            shape,
            group_size,
            data_offset: cumulative_offset,
            data_size,
        });
        cumulative_offset = tensor_end;
    }

    Ok(ParsedHfq {
        arch_id,
        metadata_json,
        tensors,
        tensor_map,
    })
}

impl HfqFile {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let file_len = hfq_usize(file.metadata()?.len(), "file length")?;
        if file_len < HFQ_HEADER_SIZE {
            return Err(invalid_hfq(format!(
                "{}: truncated HFQ header: {file_len} bytes (need {HFQ_HEADER_SIZE})",
                path.display()
            )));
        }
        let mmap = unsafe { Mmap::map(&file)? };
        // Sequential access hint: helps the kernel readahead and drop pages sooner.
        #[cfg(unix)]
        {
            mmap.advise(memmap2::Advice::Sequential).ok();
            // Also advise the file descriptor for the data region.
            use std::os::unix::io::AsRawFd;
            unsafe {
                libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_SEQUENTIAL);
            }
        }

        let ParsedHfq {
            arch_id,
            metadata_json,
            tensors,
            tensor_map,
        } = parse_hfq_bytes(&mmap)
            .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))?;

        Ok(Self {
            _file: file,
            path: path.to_path_buf(),
            mmap: Some(mmap), arch_id, metadata_json, tensors, tensor_map,
            pread_buf: std::cell::RefCell::new(Vec::new()),
        })
    }

    /// Drop the mmap to free the virtual address mapping. After this call,
    /// `tensor_data()` returns `None` and all reads go through `tensor_data_pread()`.
    ///
    /// On unified-memory APUs (Strix Halo, Steam Deck, etc.), GPU and CPU
    /// share physical RAM. Keeping the mmap alive while hipMalloc copies
    /// tensor data into GPU buffers doubles memory consumption (mmap pages
    /// + GPU copy both resident). Dropping the mmap after header/index
    /// parsing lets the kernel reclaim those pages.
    ///
    /// On discrete-GPU systems this is unnecessary (GPU VRAM is separate),
    /// so callers should only invoke this when UMA is detected.
    pub fn drop_mmap(&mut self) {
        self.mmap = None;
    }

    /// Path the HFQ file was opened from. The weight pager uses this to
    /// open its own file handle for paged reads — keeping the pager's
    /// transport independent of this struct's lifetime / mmap.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The upstream HuggingFace Jinja `chat_template` baked into this
    /// .hfq's `tokenizer_config` metadata. `None` when the source model
    /// did not ship a chat_template (rare for instruct models, common
    /// for base models). The runtime renders this when present so prompt
    /// framing matches the model's training-time expectation; absent or
    /// failing renders fall back to the hand-rolled `prompt_frame` path.
    pub fn chat_template(&self) -> Option<String> {
        let meta: serde_json::Value = serde_json::from_str(&self.metadata_json).ok()?;
        meta.get("tokenizer_config")?
            .get("chat_template")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// Resolve a tensor name, trying common prefix variants.
    ///
    /// Qwen3.5 safetensors-converted files store tensors under
    /// `model.language_model.layers.N.` while the canonical GGUF-derived
    /// hipfire-quantize path produces `model.layers.N.`. Callers consistently
    /// pass one prefix style; this helper tries the exact name first, then
    /// strips or adds the `model.language_model.` prefix so a model file
    /// from either pipeline loads cleanly. Returns `None` only when no
    /// variant matches — the per-callsite `?` early-return is preserved.
    fn resolve_idx(&self, name: &str) -> Option<usize> {
        if let Some(&idx) = self.tensor_map.get(name) {
            return Some(idx);
        }
        // Strip "model.language_model." → "model."
        if let Some(rest) = name.strip_prefix("model.language_model.") {
            let short = format!("model.{rest}");
            if let Some(&idx) = self.tensor_map.get(&short) {
                return Some(idx);
            }
        }
        // Add "model.language_model." prefix: "model.X" → "model.language_model.X"
        if let Some(rest) = name.strip_prefix("model.") {
            let long = format!("model.language_model.{rest}");
            if let Some(&idx) = self.tensor_map.get(&long) {
                return Some(idx);
            }
        }
        // Try with `model.` / `model.language_model.` added when name has no
        // `model.` prefix at all (e.g. `lm_head.weight`).
        if !name.starts_with("model.") {
            let with_model = format!("model.{name}");
            if let Some(&idx) = self.tensor_map.get(&with_model) {
                return Some(idx);
            }
            let with_lm = format!("model.language_model.{name}");
            if let Some(&idx) = self.tensor_map.get(&with_lm) {
                return Some(idx);
            }
        }
        None
    }

    /// Look up a tensor's metadata (name, quant_type, shape, byte offset/size)
    /// without copying its data. The weight pager calls this at load time to
    /// register byte ranges without forcing eager VRAM allocation.
    pub fn find_tensor_info(&self, name: &str) -> Option<&HfqTensorInfo> {
        let idx = self.resolve_idx(name)?;
        Some(&self.tensors[idx])
    }

    pub fn tensor_data(&self, name: &str) -> Option<(&HfqTensorInfo, &[u8])> {
        let idx = self.resolve_idx(name)?;
        let info = &self.tensors[idx];
        debug_assert!(
            self.mmap.is_some(),
            "tensor_data() called after drop_mmap() — use tensor_data_vec() or tensor_data_pread() instead (tensor: {name})"
        );
        let mmap = self.mmap.as_ref()?;
        Some((info, &mmap[info.data_offset..info.data_offset + info.data_size]))
    }

    /// Read tensor data via pread into a reusable buffer, then FADV_DONTNEED
    /// the file range. On unified-memory APUs (Strix Halo etc.), mmap pages
    /// can't be evicted while the mapping exists, so pread + fadvise is the
    /// only way to prevent page cache from starving hipMalloc.
    ///
    /// Returns (info, guard) where guard derefs to `&[u8]`. The buffer is
    /// reused across calls — the previous data is overwritten.
    #[cfg(unix)]
    pub fn tensor_data_pread(&self, name: &str) -> Option<(&HfqTensorInfo, std::cell::Ref<'_, Vec<u8>>)> {
        use std::os::unix::io::AsRawFd;
        let idx = self.resolve_idx(name)?;
        let info = &self.tensors[idx];
        let fd = self._file.as_raw_fd();
        {
            let mut buf = self.pread_buf.borrow_mut();
            buf.resize(info.data_size, 0);
            let mut total_read = 0usize;
            while total_read < info.data_size {
                let n = unsafe {
                    libc::pread(
                        fd,
                        buf[total_read..].as_mut_ptr() as *mut libc::c_void,
                        info.data_size - total_read,
                        (info.data_offset + total_read) as libc::off_t,
                    )
                };
                if n <= 0 { break; }
                total_read += n as usize;
            }
            // Evict these pages from cache — works because pread doesn't hold a mapping.
            fadvise_dontneed(fd, info.data_offset, info.data_size);
        }
        Some((info, self.pread_buf.borrow()))
    }

    /// Non-unix fallback: just delegates to mmap-based tensor_data.
    #[cfg(not(unix))]
    pub fn tensor_data_pread(&self, name: &str) -> Option<(&HfqTensorInfo, &[u8])> {
        self.tensor_data(name)
    }

    /// Read tensor data using the best available path:
    /// - Unix with pread support: pread + fadvise_dontneed (avoids page cache buildup)
    /// - Fallback: mmap slice (returns None if mmap was dropped)
    ///
    /// Returns owned Vec<u8> to avoid lifetime issues with the pread RefCell.
    pub fn tensor_data_vec(&self, name: &str) -> Option<(&HfqTensorInfo, Vec<u8>)> {
        let idx = self.resolve_idx(name)?;
        let info = &self.tensors[idx];

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self._file.as_raw_fd();
            let mut buf = vec![0u8; info.data_size];
            let mut total_read = 0usize;
            while total_read < info.data_size {
                let n = unsafe {
                    libc::pread(
                        fd,
                        buf[total_read..].as_mut_ptr() as *mut libc::c_void,
                        info.data_size - total_read,
                        (info.data_offset + total_read) as libc::off_t,
                    )
                };
                if n <= 0 { break; }
                total_read += n as usize;
            }
            fadvise_dontneed(fd, info.data_offset, info.data_size);
            return Some((info, buf));
        }

        #[cfg(not(unix))]
        {
            let mmap = self.mmap.as_ref()?;
            Some((info, mmap[info.data_offset..info.data_offset + info.data_size].to_vec()))
        }
    }

    /// Release page cache for a byte range. Only works if the range is NOT mmap'd.
    #[allow(dead_code)]
    pub fn drop_pages_range(&self, offset: usize, len: usize) {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            fadvise_dontneed(self._file.as_raw_fd(), offset, len);
        }
        #[cfg(not(unix))]
        { let _ = (offset, len); }
    }

    /// Return the (start_offset, end_offset) byte range covering all tensors
    /// whose name contains `prefix.` (e.g. "layers.5.").
    #[allow(dead_code)]
    pub fn layer_data_range(&self, prefix: &str) -> Option<(usize, usize)> {
        let needle = format!("{prefix}.");
        let mut lo = usize::MAX;
        let mut hi = 0usize;
        for t in &self.tensors {
            if t.name.contains(&needle) {
                lo = lo.min(t.data_offset);
                hi = hi.max(t.data_offset + t.data_size);
            }
        }
        if lo < hi { Some((lo, hi)) } else { None }
    }

    fn find_tensor(&self, name: &str) -> Option<&HfqTensorInfo> {
        self.resolve_idx(name).map(|i| &self.tensors[i])
    }

    /// Returns the name of the first tensor whose `quant_type` matches `qt`,
    /// or `None` if none match. Used by the daemon's DFlash-refusal guard to
    /// detect MQ3/MQ2 body weights without iterating the index outside this
    /// module.
    pub fn first_tensor_with_quant_type(&self, qt: u8) -> Option<&str> {
        self.tensors
            .iter()
            .find(|t| t.quant_type == qt)
            .map(|t| t.name.as_str())
    }

    /// All tensors in index order. For tools that scan the file (e.g.
    /// dump_norms, quant_quality_mse, compare_hfq) — the engine itself
    /// looks tensors up by name via `find_tensor_info` /
    /// `tensor_data_vec`.
    pub fn tensors(&self) -> &[HfqTensorInfo] {
        &self.tensors
    }
}

// ─── Config from HFQ metadata ───────────────────────────────────────────────

pub fn config_from_hfq(hfq: &HfqFile) -> Option<LlamaConfig> {
    let meta: serde_json::Value = serde_json::from_str(&hfq.metadata_json).ok()?;
    let config = meta.get("config")?;

    let arch_str = config.get("model_type")?.as_str()?;
    let arch = match arch_str {
        "llama" => ModelArch::Llama,
        "qwen3" | "qwen2" => ModelArch::Qwen3,
        _ => ModelArch::Llama,
    };

    let dim = config.get("hidden_size")?.as_u64()? as usize;
    let n_layers = config.get("num_hidden_layers")?.as_u64()? as usize;
    let n_heads = config.get("num_attention_heads")?.as_u64()? as usize;
    let n_kv_heads = config.get("num_key_value_heads")
        .and_then(|v| v.as_u64())
        .unwrap_or(n_heads as u64) as usize;
    let hidden_dim = config.get("intermediate_size")?.as_u64()? as usize;
    let vocab_size = config.get("vocab_size")?.as_u64()? as usize;
    let norm_eps = config.get("rms_norm_eps")
        .and_then(|v| v.as_f64())
        .unwrap_or(1e-5) as f32;
    let max_seq_len = config.get("max_position_embeddings")
        .and_then(|v| v.as_u64())
        .unwrap_or(2048) as usize;
    let rope_freq_base = config.get("rope_theta")
        .and_then(|v| v.as_f64())
        .unwrap_or(10000.0) as f32;

    let has_qk_norm = hfq.find_tensor("model.layers.0.self_attn.q_norm.weight").is_some();

    let head_dim = config.get("head_dim")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(dim / n_heads);

    let bos_token = config.get("bos_token_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let eos_token = config.get("eos_token_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as u32;

    Some(LlamaConfig {
        arch, dim, hidden_dim, n_layers, n_heads, n_kv_heads, vocab_size,
        head_dim, norm_eps, max_seq_len, rope_freq_base,
        bos_token, eos_token,
        has_qk_norm,
    })
}

// ─── Weight Loading ─────────────────────────────────────────────────────────

/// Load a tensor as F32 on GPU (for norms, embeddings).
fn load_f16_tensor(hfq: &HfqFile, gpu: &mut Gpu, st_name: &str, shape: &[usize]) -> HipResult<GpuTensor> {
    let (info, data) = hfq.tensor_data(st_name)
        .unwrap_or_else(|| panic!("tensor not found: {st_name}"));
    info.expect_shape(shape)?;

    let f32_data: Vec<f32> = match info.quant_type {
        1 => { // F16
            data.chunks_exact(2)
                .map(|c| f16_to_f32(u16::from_le_bytes([c[0], c[1]])))
                .collect()
        }
        2 => { // F32
            data.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        }
        _ => panic!("expected F16/F32 tensor for {st_name}, got quant_type={}", info.quant_type),
    };

    gpu.upload_f32(&f32_data, shape)
}

/// Load a weight tensor (quantized or F16) onto GPU.
fn load_weight_tensor(hfq: &HfqFile, gpu: &Gpu, st_name: &str, m: usize, k: usize) -> HipResult<WeightTensor> {
    let (info, data) = hfq.tensor_data(st_name)
        .unwrap_or_else(|| panic!("tensor not found: {st_name}"));
    info.expect_shape(&[m, k])?;

    // Phase A Stage A — AWQ sidecar lookup. The quantizer emits per-tensor
    // sidecars named `<weight_name>.awq_scale.weight` (1D F16, length K)
    // alongside MQ4-quantized weights. The forward path uses these to apply
    // `x /= awq_scale` before the rotation kernel, completing the AWQ
    // math `(W·s) · (x/s) = W·x`. Backward-compatible: when no sidecar
    // exists (the common case for pre-Stage-A .hfq files), `awq_scale`
    // stays None and the runtime behaves identically to before.
    //
    // Naming convention: replace `.weight` with `.awq_scale.weight` so the
    // sidecar gets stored as an F16 1D tensor that the loader can detect
    // by name. Matches hipfire-quantize's emit pattern.
    let load_awq_scale = || -> Option<GpuTensor> {
        let sidecar_name = match st_name.strip_suffix(".weight") {
            Some(stem) => format!("{stem}.awq_scale.weight"),
            None => format!("{st_name}.awq_scale.weight"),
        };
        let (sc_info, sc_data) = hfq.tensor_data(&sidecar_name)?;
        // Must be 1D F16, length K. Quant type 1 = F16 per the existing
        // load_f16_tensor path (quant_type field documented at line ~31).
        if sc_info.quant_type != 1 {
            eprintln!("warning: AWQ sidecar {sidecar_name} has quant_type={} (expected 1=F16); skipping", sc_info.quant_type);
            return None;
        }
        if sc_info.shape.len() != 1 || sc_info.shape[0] != k as u32 {
            eprintln!("warning: AWQ sidecar {sidecar_name} shape mismatch ({:?} vs expected [{}]); skipping", sc_info.shape, k);
            return None;
        }
        // Convert F16 → F32 on host before upload, so the kernel receives
        // a `const float*` and doesn't need <hip/hip_fp16.h>. The scale
        // vector is small (K ≤ ~12288 elements, ~48 KB peak), so the
        // 2× memory cost on GPU vs raw F16 is negligible.
        let f32_data: Vec<f32> = sc_data
            .chunks_exact(2)
            .map(|c| crate::llama::f16_to_f32(u16::from_le_bytes([c[0], c[1]])))
            .collect();
        let f32_bytes: Vec<u8> = f32_data.iter()
            .flat_map(|&v| v.to_le_bytes())
            .collect();
        gpu.upload_raw(&f32_bytes, &[f32_bytes.len()]).ok()
    };

    let mut wt = match info.quant_type {
        0 => { // Q4F16G64
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::Q4F16G64, m, k, row_stride: 0, awq_scale: None })
        }
        3 => { // Q8F16 — same block format as GGML Q8_0 (34 bytes per 32 elements)
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::Q8_0, m, k, row_stride: 0, awq_scale: None })
        }
        4 => { // Q4_K — GGML-compatible Q4_K blocks (144 bytes per 256 elements)
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::Q4K, m, k, row_stride: 0, awq_scale: None })
        }
        5 => { // Q8HFQ — split-metadata layout (scales then values, 128B-aligned rows)
            let n_groups = k / 32;
            let raw_row = n_groups * 2 + k;
            let row_stride = (raw_row + 127) & !127;
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::Q8HFQ, m, k, row_stride, awq_scale: None })
        }
        6 => { // HFQ4-G256 — flat 4-bit, 136 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFQ4G256, m, k, row_stride: 0, awq_scale: None })
        }
        7 => { // HFQ4-G128 — flat 4-bit, 72 bytes per 128 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFQ4G128, m, k, row_stride: 0, awq_scale: None })
        }
        8 => { // HFQ6-G256 — 6-bit, 200 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFQ6G256, m, k, row_stride: 0, awq_scale: None })
        }
        9 => { // HFQ2-G256 — flat 2-bit, 72 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFQ2G256, m, k, row_stride: 0, awq_scale: None })
        }
        10 => { // HFQ2-G128 — flat 2-bit, 40 bytes per 128 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFQ2G128, m, k, row_stride: 0, awq_scale: None })
        }
        11 => { // HFQ3-G256 — flat 3-bit, 104 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFQ3G256, m, k, row_stride: 0, awq_scale: None })
        }
        12 => { // HFQ3-G128 — flat 3-bit, 56 bytes per 128 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFQ3G128, m, k, row_stride: 0, awq_scale: None })
        }
        13 => { // MQ4-G256 — MagnumQuant FWHT-rotated 4-bit
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MQ4G256, m, k, row_stride: 0, awq_scale: None })
        }
        14 => { // MQ8-G256 — MagnumQuant FWHT-rotated symmetric INT8, dp4a
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MQ8G256, m, k, row_stride: 0, awq_scale: None })
        }
        15 => { // MQ6-G256 — MagnumQuant FWHT-rotated 6-bit, 200 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MQ6G256, m, k, row_stride: 0, awq_scale: None })
        }
        17 => { // MQ3-G256 — MagnumQuant FWHT-rotated 3-bit, 104 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MQ3G256, m, k, row_stride: 0, awq_scale: None })
        }
        18 => { // MQ2-G256 — MagnumQuant FWHT-rotated 2-bit, 72 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MQ2G256, m, k, row_stride: 0, awq_scale: None })
        }
        19 => { // MQ2-G256-Lloyd — 2-bit + 4-entry fp16 codebook, 72 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MQ2G256Lloyd, m, k, row_stride: 0, awq_scale: None })
        }
        20 => { // MQ3-G256-Lloyd — 3-bit + 8-entry fp16 codebook, 112 bytes per 256 elements
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MQ3G256Lloyd, m, k, row_stride: 0, awq_scale: None })
        }
        21 => { // HFP4G32 — E2M1 + UE8M0 g32 + FP16 row scale.
                // Per-row hdr 16 B + (K/32) blocks × 17 B. See docs/quant-formats/hfp4.md.
                // K%256 — kernel constraint (gemv_hfp4g32 in dispatch.rs);
                // refuse here so a stale or externally-quantized file fails at
                // load instead of panicking on first dispatch.
            assert!(k % 256 == 0, "HFP4G32 v1 weight {st_name} has K={k} but kernel requires K%256==0");
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::HFP4G32, m, k, row_stride: 0, awq_scale: None })
        }
        24 => { // MFP4G32 — HFP4G32 + offline FWHT rotation (drop-in MQ4 replacement).
                // Same byte layout as qtype 21; format_flags=0x05 in row hdr.
                // See docs/quant-formats/hfp4.md.
            assert!(k % 256 == 0, "MFP4G32 weight {st_name} has K={k} but kernel + FWHT both require K%256==0");
            let buf = gpu.upload_raw(data, &[data.len()])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::MFP4G32, m, k, row_stride: 0, awq_scale: None })
        }
        1 => { // F16 — dequant to F32 for F32 GEMV
            let f32_data: Vec<f32> = data.chunks_exact(2)
                .map(|c| f16_to_f32(u16::from_le_bytes([c[0], c[1]])))
                .collect();
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(f32_data.as_ptr() as *const u8, f32_data.len() * 4)
            };
            let buf = gpu.upload_raw(bytes, &[m, k])?;
            Ok(WeightTensor { buf, gpu_dtype: DType::F32, m, k, row_stride: 0, awq_scale: None })
        }
        _ => panic!("unsupported quant_type {} for weight {st_name}", info.quant_type),
    }?;
    // Centralized AWQ sidecar attachment. Replaces the prior per-arm
    // inline `load_awq_scale()` calls at the qt=13 / qt=17 arms — those
    // were the only loaders touching `awq_scale` and missing arms (qt=15
    // MQ6, qt=18 MQ2, qt=19/20 Lloyd, qt=24 MFP4) would silently drop
    // sidecars if added later. Routed through `DType::supports_awq_sidecar`
    // so future widening is a single helper edit, not a scattered
    // per-loader hunt. See dispatch.rs for the allow-list rationale.
    if wt.gpu_dtype.supports_awq_sidecar() {
        wt.awq_scale = load_awq_scale();
    }
    Ok(wt)
}

/// Load LLaMA weights from an HFQ file onto GPU.
pub fn load_weights_hfq(
    hfq: &HfqFile,
    config: &LlamaConfig,
    gpu: &mut Gpu,
) -> HipResult<LlamaWeights> {
    eprintln!("  loading token_embd...");
    let embd_info = hfq.tensor_data("model.embed_tokens.weight")
        .expect("embed_tokens not found");
    embd_info.0.expect_shape(&[config.vocab_size, config.dim])?;
    let (token_embd, embd_fmt) = if embd_info.0.quant_type == 4 {
        // Q4_K: upload raw, use Q4K embedding lookup at inference
        eprintln!("    (Q4K raw, {} MB)", embd_info.1.len() / 1_000_000);
        (gpu.upload_raw(embd_info.1, &[embd_info.1.len()])?, EmbeddingFormat::Q4K)
    } else if embd_info.0.quant_type == 6 {
        eprintln!("    (HFQ4-G256 raw, {} MB)", embd_info.1.len() / 1_000_000);
        (gpu.upload_raw(embd_info.1, &[embd_info.1.len()])?, EmbeddingFormat::HFQ4G256)
    } else if embd_info.0.quant_type == 7 {
        eprintln!("    (HFQ4-G128 raw, {} MB)", embd_info.1.len() / 1_000_000);
        (gpu.upload_raw(embd_info.1, &[embd_info.1.len()])?, EmbeddingFormat::HFQ4G128)
    } else if embd_info.0.quant_type == 3 {
        // Q8F16: upload raw, use Q8 embedding lookup at inference
        eprintln!("    (Q8 raw, {} MB)", embd_info.1.len() / 1_000_000);
        (gpu.upload_raw(embd_info.1, &[embd_info.1.len()])?, EmbeddingFormat::Q8_0)
    } else {
        (load_f16_tensor(hfq, gpu, "model.embed_tokens.weight",
            &[config.vocab_size, config.dim])?, EmbeddingFormat::F32)
    };

    eprintln!("  loading output_norm...");
    let output_norm = load_f16_tensor(hfq, gpu, "model.norm.weight", &[config.dim])?;

    eprintln!("  loading output...");
    let output = if hfq.find_tensor("lm_head.weight").is_some() {
        load_weight_tensor(hfq, gpu, "lm_head.weight", config.vocab_size, config.dim)?
    } else {
        // Tied embeddings — reuse token_embd as output weights (F32 for GEMV)
        let data = hfq.tensor_data("model.embed_tokens.weight").unwrap().1;
        let f32_data: Vec<f32> = data.chunks_exact(2)
            .map(|c| f16_to_f32(u16::from_le_bytes([c[0], c[1]])))
            .collect();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(f32_data.as_ptr() as *const u8, f32_data.len() * 4)
        };
        let buf = gpu.upload_raw(bytes, &[config.vocab_size, config.dim])?;
        WeightTensor { buf, gpu_dtype: DType::F32, m: config.vocab_size, k: config.dim, row_stride: 0, awq_scale: None }
    };

    let mut layers = Vec::with_capacity(config.n_layers);
    for i in 0..config.n_layers {
        eprintln!("  loading layer {i}/{} ...", config.n_layers);
        let p = format!("model.layers.{i}");
        let kv_dim = config.n_kv_heads * config.head_dim;
        let q_out_dim = config.n_heads * config.head_dim;

        let layer = LayerWeights {
            attn_norm: load_f16_tensor(hfq, gpu,
                &format!("{p}.input_layernorm.weight"), &[config.dim])?,
            wq: load_weight_tensor(hfq, gpu,
                &format!("{p}.self_attn.q_proj.weight"), q_out_dim, config.dim)?,
            wk: load_weight_tensor(hfq, gpu,
                &format!("{p}.self_attn.k_proj.weight"), kv_dim, config.dim)?,
            wv: load_weight_tensor(hfq, gpu,
                &format!("{p}.self_attn.v_proj.weight"), kv_dim, config.dim)?,
            wo: load_weight_tensor(hfq, gpu,
                &format!("{p}.self_attn.o_proj.weight"), config.dim, q_out_dim)?,
            q_norm: if config.has_qk_norm {
                Some(load_f16_tensor(hfq, gpu,
                    &format!("{p}.self_attn.q_norm.weight"), &[config.head_dim])?)
            } else { None },
            k_norm: if config.has_qk_norm {
                Some(load_f16_tensor(hfq, gpu,
                    &format!("{p}.self_attn.k_norm.weight"), &[config.head_dim])?)
            } else { None },
            ffn_norm: load_f16_tensor(hfq, gpu,
                &format!("{p}.post_attention_layernorm.weight"), &[config.dim])?,
            w_gate: load_weight_tensor(hfq, gpu,
                &format!("{p}.mlp.gate_proj.weight"), config.hidden_dim, config.dim)?,
            w_up: load_weight_tensor(hfq, gpu,
                &format!("{p}.mlp.up_proj.weight"), config.hidden_dim, config.dim)?,
            w_down: load_weight_tensor(hfq, gpu,
                &format!("{p}.mlp.down_proj.weight"), config.dim, config.hidden_dim)?,
        };
        layers.push(layer);
    }

    Ok(LlamaWeights { token_embd, embd_format: embd_fmt, output_norm, output, layers })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTensor<'a> {
        name: &'a str,
        quant_type: u8,
        shape: &'a [u32],
        group_size: u32,
        data: &'a [u8],
    }

    fn build_hfq(metadata: &str, tensors: &[TestTensor<'_>]) -> Vec<u8> {
        let mut index = Vec::new();
        index.extend_from_slice(&(tensors.len() as u32).to_le_bytes());
        for tensor in tensors {
            index.extend_from_slice(&(tensor.name.len() as u16).to_le_bytes());
            index.extend_from_slice(tensor.name.as_bytes());
            index.push(tensor.quant_type);
            index.push(tensor.shape.len() as u8);
            for dim in tensor.shape {
                index.extend_from_slice(&dim.to_le_bytes());
            }
            index.extend_from_slice(&tensor.group_size.to_le_bytes());
            index.extend_from_slice(&(tensor.data.len() as u64).to_le_bytes());
        }

        let metadata_offset = HFQ_HEADER_SIZE as u64;
        let data_offset = metadata_offset + metadata.len() as u64 + index.len() as u64;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(HFQ_MAGIC);
        bytes.extend_from_slice(&HFQ_VERSION_SUPPORTED.to_le_bytes());
        bytes.extend_from_slice(&5u32.to_le_bytes());
        bytes.extend_from_slice(&(tensors.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&metadata_offset.to_le_bytes());
        bytes.extend_from_slice(&data_offset.to_le_bytes());
        bytes.extend_from_slice(metadata.as_bytes());
        bytes.extend_from_slice(&index);
        for tensor in tensors {
            bytes.extend_from_slice(tensor.data);
        }
        bytes
    }

    fn parse_error(bytes: &[u8]) -> String {
        match parse_hfq_bytes(bytes) {
            Ok(_) => panic!("malformed HFQ unexpectedly parsed"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_valid_header_index_and_payload_ranges() {
        let metadata = r#"{"config":{"model_type":"qwen3"}}"#;
        let mq4_data = vec![0u8; 136];
        let f16_data = vec![0u8; 8];
        let tensors = [
            TestTensor {
                name: "model.embed_tokens.weight",
                quant_type: 13,
                shape: &[1, 256],
                group_size: 256,
                data: &mq4_data,
            },
            TestTensor {
                name: "lm_head.weight",
                quant_type: 1,
                shape: &[4],
                group_size: 0,
                data: &f16_data,
            },
        ];
        let bytes = build_hfq(metadata, &tensors);
        let parsed = parse_hfq_bytes(&bytes).expect("valid HFQ");

        assert_eq!(parsed.arch_id, 5);
        assert_eq!(parsed.metadata_json, metadata);
        assert_eq!(parsed.tensors.len(), 2);
        assert_eq!(parsed.tensors[0].shape, vec![1, 256]);
        assert_eq!(parsed.tensors[0].quant_type, 13);
        assert_eq!(
            parsed.tensors[1].data_offset,
            parsed.tensors[0].data_offset + 136
        );
        assert_eq!(
            &bytes[parsed.tensors[1].data_offset..parsed.tensors[1].data_offset + 8],
            &[0; 8]
        );
        assert_eq!(parsed.tensor_map["lm_head.weight"], 1);
    }

    #[test]
    fn validates_registered_quant_layout_sizes() {
        let cases: &[(u8, &[u32], u32, usize)] = &[
            (0, &[65], 64, 72),
            (1, &[3], 0, 6),
            (2, &[3], 0, 12),
            (3, &[33], 32, 68),
            (4, &[257], 256, 288),
            (5, &[2, 256], 32, 768),
            (6, &[1, 256], 256, 136),
            (7, &[1, 128], 128, 72),
            (8, &[1, 256], 256, 200),
            (9, &[1, 256], 256, 72),
            (10, &[1, 128], 128, 40),
            (11, &[1, 256], 256, 104),
            (12, &[1, 128], 128, 56),
            (13, &[1, 256], 256, 136),
            (14, &[1, 256], 256, 258),
            (15, &[1, 256], 256, 200),
            (16, &[3], 0, 6),
            (17, &[1, 256], 256, 104),
            (18, &[1, 256], 256, 72),
            (19, &[1, 256], 256, 72),
            (20, &[1, 256], 256, 112),
            (21, &[2, 256], 32, 304),
            (24, &[2, 512], 32, 576),
        ];
        for &(quant_type, shape, group_size, data_size) in cases {
            validate_hfq_tensor_layout("weight", quant_type, shape, group_size, data_size)
                .unwrap_or_else(|err| panic!("qt={quant_type} shape={shape:?}: {err}"));
        }
    }

    #[test]
    fn rejects_quant_layout_metadata_and_payload_mismatches() {
        let error = validate_hfq_tensor_layout("weight", 13, &[1, 256], 128, 136)
            .unwrap_err()
            .to_string();
        assert!(error.contains("group_size 128, expected 256"));

        let error = validate_hfq_tensor_layout("weight", 1, &[4], 0, 6)
            .unwrap_err()
            .to_string();
        assert!(error.contains("payload is 6 bytes, expected 8"));

        assert!(validate_hfq_tensor_layout("weight", 5, &[256], 32, 384)
            .unwrap_err()
            .to_string()
            .contains("must be 2D"));
        assert!(validate_hfq_tensor_layout("weight", 21, &[1, 384], 32, 220)
            .unwrap_err()
            .to_string()
            .contains("not divisible by 256"));
        assert!(validate_hfq_tensor_layout("weight", 22, &[1, 256], 32, 136)
            .unwrap_err()
            .to_string()
            .contains("unsupported quant_type 22"));
    }

    #[test]
    fn consumer_shape_contract_rejects_config_mismatch() {
        let info = HfqTensorInfo {
            name: "model.layers.0.mlp.gate_proj.weight".into(),
            quant_type: 13,
            shape: vec![64, 256],
            group_size: 256,
            data_offset: 0,
            data_size: 64 * 136,
        };

        info.expect_shape(&[64, 256]).expect("exact shape");
        info.expect_numel(64 * 256).expect("flattened element count");

        let error = info.expect_shape(&[32, 512]).unwrap_err().to_string();
        assert!(error.contains("shape mismatch"));
        assert!(error.contains("[64, 256]"));
        assert!(error.contains("[32, 512]"));

        let error = info.expect_numel(64 * 255).unwrap_err().to_string();
        assert!(error.contains("element-count mismatch"));
    }

    #[test]
    fn rejects_truncated_header_bad_magic_and_future_version() {
        assert!(parse_error(&[0; HFQ_HEADER_SIZE - 1]).contains("truncated HFQ header"));

        let mut bytes = build_hfq("{}", &[]);
        bytes[0] = b'X';
        assert!(parse_error(&bytes).contains("invalid HFQ magic"));

        let mut bytes = build_hfq("{}", &[]);
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert!(parse_error(&bytes).contains("unsupported HFQ version 2"));
    }

    #[test]
    fn rejects_invalid_offsets_and_metadata() {
        let mut bytes = build_hfq("{}", &[]);
        bytes[16..24].copy_from_slice(&31u64.to_le_bytes());
        assert!(parse_error(&bytes).contains("overlaps the 32-byte header"));

        let mut bytes = build_hfq("{}", &[]);
        let past_end = bytes.len() as u64 + 1;
        bytes[24..32].copy_from_slice(&past_end.to_le_bytes());
        assert!(parse_error(&bytes).contains("exceeds file length"));

        let bytes = build_hfq("{", &[]);
        assert!(parse_error(&bytes).contains("invalid HFQ metadata JSON"));

        let bytes = build_hfq("[]", &[]);
        assert!(parse_error(&bytes).contains("metadata JSON must be an object"));
    }

    #[test]
    fn rejects_header_index_count_mismatch_and_impossible_capacity() {
        let mut bytes = build_hfq("{}", &[]);
        bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
        assert!(parse_error(&bytes).contains("tensor count mismatch"));

        let mut bytes = build_hfq("{}", &[]);
        bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        let index_count_offset = HFQ_HEADER_SIZE + 2;
        bytes[index_count_offset..index_count_offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_error(&bytes).contains("cannot fit in the declared index"));
    }

    #[test]
    fn rejects_duplicate_or_non_utf8_tensor_names() {
        let duplicate = [
            TestTensor {
                name: "weight",
                quant_type: 1,
                shape: &[1],
                group_size: 0,
                data: &[0, 0],
            },
            TestTensor {
                name: "weight",
                quant_type: 1,
                shape: &[1],
                group_size: 0,
                data: &[0, 0],
            },
        ];
        assert!(parse_error(&build_hfq("{}", &duplicate)).contains("duplicate HFQ tensor name"));

        let tensor = [TestTensor {
            name: "weight",
            quant_type: 1,
            shape: &[1],
            group_size: 0,
            data: &[0, 0],
        }];
        let mut bytes = build_hfq("{}", &tensor);
        let first_name_byte = HFQ_HEADER_SIZE + 2 + 4 + 2;
        bytes[first_name_byte] = 0xff;
        assert!(parse_error(&bytes).contains("name is not UTF-8"));
    }

    #[test]
    fn rejects_truncated_index_and_payload() {
        let tensor = [TestTensor {
            name: "weight",
            quant_type: 1,
            shape: &[1],
            group_size: 0,
            data: &[0, 0],
        }];
        let mut bytes = build_hfq("{}", &tensor);
        let data_offset = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        bytes[24..32].copy_from_slice(&(data_offset - 1).to_le_bytes());
        assert!(parse_error(&bytes).contains("truncated HFQ tensor data size"));

        let mut bytes = build_hfq("{}", &tensor);
        bytes.pop();
        assert!(parse_error(&bytes).contains("data range"));
    }
}
