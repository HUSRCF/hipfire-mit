// SPDX-License-Identifier: MIT
//! Image loading and preprocessing for the Qwen3.5-VL vision encoder.

use std::fmt;
use std::path::Path;

const CHANNELS: usize = 3;
const MIN_PIXELS: usize = 56 * 56;
const MAX_PIXELS: usize = 14 * 14 * 4 * 1280;
const MAX_ASPECT_RATIO: usize = 200;

/// One checked, model-consumable representation shared by every VL frontend.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedImage {
    patches: Vec<f32>,
    resized_height: usize,
    resized_width: usize,
    grid_height: usize,
    grid_width: usize,
    visual_tokens: usize,
}

impl PreparedImage {
    pub fn patches(&self) -> &[f32] {
        &self.patches
    }

    pub fn resized_height(&self) -> usize {
        self.resized_height
    }

    pub fn resized_width(&self) -> usize {
        self.resized_width
    }

    pub fn grid_height(&self) -> usize {
        self.grid_height
    }

    pub fn grid_width(&self) -> usize {
        self.grid_width
    }

    pub fn visual_tokens(&self) -> usize {
        self.visual_tokens
    }

    pub fn patch_count(&self) -> usize {
        self.grid_height * self.grid_width
    }
}

#[derive(Debug)]
pub enum ImageInputError {
    Decode {
        path: String,
        source: image::ImageError,
    },
    InvalidConfiguration(&'static str),
    InvalidDimensions(String),
    SizeOverflow(&'static str),
    LayoutMismatch {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for ImageInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode { path, source } => write!(f, "failed to decode image {path}: {source}"),
            Self::InvalidConfiguration(message) => {
                write!(f, "invalid image preprocessing configuration: {message}")
            }
            Self::InvalidDimensions(message) => write!(f, "invalid image dimensions: {message}"),
            Self::SizeOverflow(stage) => write!(f, "image size overflow while computing {stage}"),
            Self::LayoutMismatch { expected, actual } => write!(
                f,
                "image tensor layout mismatch: expected {expected} elements, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ImageInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn checked_mul(a: usize, b: usize, stage: &'static str) -> Result<usize, ImageInputError> {
    a.checked_mul(b).ok_or(ImageInputError::SizeOverflow(stage))
}

fn checked_area(height: usize, width: usize) -> Result<usize, ImageInputError> {
    checked_mul(height, width, "image area")
}

fn rounded_multiple(value: usize, factor: usize) -> Result<usize, ImageInputError> {
    let numerator = (value as u128)
        .checked_add((factor / 2) as u128)
        .ok_or(ImageInputError::SizeOverflow("rounded resize dimension"))?;
    let units = numerator / factor as u128;
    let result = units
        .checked_mul(factor as u128)
        .ok_or(ImageInputError::SizeOverflow("rounded resize dimension"))?;
    usize::try_from(result).map_err(|_| ImageInputError::SizeOverflow("rounded resize dimension"))
}

fn float_multiple(raw: f64, factor: usize, round_up: bool) -> Result<usize, ImageInputError> {
    let units = if round_up {
        (raw / factor as f64).ceil()
    } else {
        (raw / factor as f64).floor()
    };
    if !units.is_finite() || units < 0.0 || units > (usize::MAX / factor) as f64 {
        return Err(ImageInputError::SizeOverflow("scaled resize dimension"));
    }
    let scaled = (units as usize)
        .checked_mul(factor)
        .ok_or(ImageInputError::SizeOverflow("scaled resize dimension"))?;
    Ok(factor.max(scaled))
}

/// Resize dimensions to a factor-aligned area accepted by the vision merger.
pub fn smart_resize(
    height: usize,
    width: usize,
    factor: usize,
    min_pixels: usize,
    max_pixels: usize,
) -> Result<(usize, usize), ImageInputError> {
    if height == 0 || width == 0 {
        return Err(ImageInputError::InvalidDimensions(
            "height and width must be positive".to_string(),
        ));
    }
    if factor == 0 {
        return Err(ImageInputError::InvalidConfiguration(
            "patch_size * spatial_merge_size must be positive",
        ));
    }
    if min_pixels == 0 || min_pixels > max_pixels {
        return Err(ImageInputError::InvalidConfiguration(
            "pixel bounds must satisfy 0 < min_pixels <= max_pixels",
        ));
    }
    let long_side = height.max(width);
    let short_side = height.min(width);
    if long_side / short_side > MAX_ASPECT_RATIO {
        return Err(ImageInputError::InvalidDimensions(format!(
            "aspect ratio exceeds {MAX_ASPECT_RATIO}:1"
        )));
    }

    let mut resized_h = rounded_multiple(height, factor)?.max(factor);
    let mut resized_w = rounded_multiple(width, factor)?.max(factor);
    let rounded_area = checked_area(resized_h, resized_w)?;
    let source_area = checked_area(height, width)?;

    if rounded_area > max_pixels {
        let beta = (source_area as f64 / max_pixels as f64).sqrt();
        resized_h = float_multiple(height as f64 / beta, factor, false)?;
        resized_w = float_multiple(width as f64 / beta, factor, false)?;
    } else if rounded_area < min_pixels {
        let beta = (min_pixels as f64 / source_area as f64).sqrt();
        resized_h = float_multiple(height as f64 * beta, factor, true)?;
        resized_w = float_multiple(width as f64 * beta, factor, true)?;
    }

    if resized_h > u32::MAX as usize || resized_w > u32::MAX as usize {
        return Err(ImageInputError::InvalidDimensions(
            "resized dimensions exceed the image decoder limit".to_string(),
        ));
    }
    let final_area = checked_area(resized_h, resized_w)?;
    if final_area > max_pixels {
        return Err(ImageInputError::InvalidDimensions(format!(
            "factor-aligned area {final_area} exceeds max_pixels={max_pixels}"
        )));
    }
    Ok((resized_h, resized_w))
}

fn normalize_decoded_image(
    img: &image::DynamicImage,
    patch_size: usize,
    spatial_merge_size: usize,
) -> Result<(Vec<f32>, usize, usize), ImageInputError> {
    let factor = checked_mul(patch_size, spatial_merge_size, "resize factor")?;
    let orig_w = img.width() as usize;
    let orig_h = img.height() as usize;
    let (height, width) = smart_resize(orig_h, orig_w, factor, MIN_PIXELS, MAX_PIXELS)?;
    let img = img.resize_exact(
        width as u32,
        height as u32,
        image::imageops::FilterType::Triangle,
    );
    let rgb = img.to_rgb8();
    let plane = checked_area(height, width)?;
    let output_len = checked_mul(CHANNELS, plane, "normalized CHW tensor")?;
    let mut out = vec![0.0f32; output_len];

    // The exported patch embedding expects [R, B, G] channel planes.
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x as u32, y as u32);
            let idx = y * width + x;
            out[idx] = pixel[0] as f32 / 127.5 - 1.0;
            out[plane + idx] = pixel[2] as f32 / 127.5 - 1.0;
            out[2 * plane + idx] = pixel[1] as f32 / 127.5 - 1.0;
        }
    }
    Ok((out, height, width))
}

/// Decode and normalize a filesystem image into CHW floats.
pub fn load_and_preprocess(
    path: &Path,
    patch_size: usize,
    spatial_merge_size: usize,
) -> Result<(Vec<f32>, usize, usize), ImageInputError> {
    let img = image::open(path).map_err(|source| ImageInputError::Decode {
        path: path.display().to_string(),
        source,
    })?;
    normalize_decoded_image(&img, patch_size, spatial_merge_size)
}

/// Convert a checked CHW tensor to the temporal patch layout consumed by ViT.
pub fn extract_patches(
    chw: &[f32],
    channels: usize,
    height: usize,
    width: usize,
    patch_size: usize,
    temporal_patch_size: usize,
) -> Result<Vec<f32>, ImageInputError> {
    if channels == 0 || patch_size == 0 || temporal_patch_size == 0 {
        return Err(ImageInputError::InvalidConfiguration(
            "channels, patch_size, and temporal_patch_size must be positive",
        ));
    }
    if height == 0 || width == 0 || height % patch_size != 0 || width % patch_size != 0 {
        return Err(ImageInputError::InvalidDimensions(format!(
            "{height}x{width} must be positive and divisible by patch_size={patch_size}"
        )));
    }
    let plane = checked_area(height, width)?;
    let expected = checked_mul(channels, plane, "CHW input tensor")?;
    if chw.len() != expected {
        return Err(ImageInputError::LayoutMismatch {
            expected,
            actual: chw.len(),
        });
    }

    let grid_height = height / patch_size;
    let grid_width = width / patch_size;
    let patch_count = checked_mul(grid_height, grid_width, "patch count")?;
    let patch_area = checked_mul(patch_size, patch_size, "patch area")?;
    let patch_channels = checked_mul(channels, patch_area, "patch channels")?;
    let patch_elems = checked_mul(temporal_patch_size, patch_channels, "temporal patch")?;
    let output_len = checked_mul(patch_count, patch_elems, "patch tensor")?;
    let mut patches = vec![0.0f32; output_len];

    for py in 0..grid_height {
        for px in 0..grid_width {
            let patch_idx = py * grid_width + px;
            let out_base = patch_idx * patch_elems;
            for t in 0..temporal_patch_size {
                for c in 0..channels {
                    for dy in 0..patch_size {
                        for dx in 0..patch_size {
                            let y = py * patch_size + dy;
                            let x = px * patch_size + dx;
                            let src_idx = c * plane + y * width + x;
                            let dst_idx = out_base
                                + t * patch_channels
                                + c * patch_area
                                + dy * patch_size
                                + dx;
                            patches[dst_idx] = chw[src_idx];
                        }
                    }
                }
            }
        }
    }
    Ok(patches)
}

fn prepare_normalized(
    chw: Vec<f32>,
    height: usize,
    width: usize,
    patch_size: usize,
    temporal_patch_size: usize,
    spatial_merge_size: usize,
) -> Result<PreparedImage, ImageInputError> {
    if spatial_merge_size == 0 {
        return Err(ImageInputError::InvalidConfiguration(
            "spatial_merge_size must be positive",
        ));
    }
    let grid_height = height / patch_size;
    let grid_width = width / patch_size;
    if grid_height % spatial_merge_size != 0 || grid_width % spatial_merge_size != 0 {
        return Err(ImageInputError::InvalidDimensions(format!(
            "patch grid {grid_height}x{grid_width} must be divisible by spatial_merge_size={spatial_merge_size}"
        )));
    }
    let merged_height = grid_height / spatial_merge_size;
    let merged_width = grid_width / spatial_merge_size;
    let visual_tokens = checked_mul(merged_height, merged_width, "visual token count")?;
    let patches = extract_patches(
        &chw,
        CHANNELS,
        height,
        width,
        patch_size,
        temporal_patch_size,
    )?;
    Ok(PreparedImage {
        patches,
        resized_height: height,
        resized_width: width,
        grid_height,
        grid_width,
        visual_tokens,
    })
}

/// Normalize a path-backed image through the shared VL input contract.
pub fn prepare_image(
    path: &Path,
    patch_size: usize,
    temporal_patch_size: usize,
    spatial_merge_size: usize,
) -> Result<PreparedImage, ImageInputError> {
    let (chw, height, width) = load_and_preprocess(path, patch_size, spatial_merge_size)?;
    prepare_normalized(
        chw,
        height,
        width,
        patch_size,
        temporal_patch_size,
        spatial_merge_size,
    )
}

/// Normalize an already-decoded image through the same VL input contract.
pub fn prepare_decoded_image(
    image: &image::DynamicImage,
    patch_size: usize,
    temporal_patch_size: usize,
    spatial_merge_size: usize,
) -> Result<PreparedImage, ImageInputError> {
    let (chw, height, width) = normalize_decoded_image(image, patch_size, spatial_merge_size)?;
    prepare_normalized(
        chw,
        height,
        width,
        patch_size,
        temporal_patch_size,
        spatial_merge_size,
    )
}
