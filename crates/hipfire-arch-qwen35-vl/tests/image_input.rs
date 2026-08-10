// SPDX-License-Identifier: MIT

use hipfire_arch_qwen35_vl::image::{
    extract_patches, prepare_decoded_image, prepare_image, smart_resize, ImageInputError,
};
use image::{DynamicImage, ImageBuffer, Rgb};

fn test_image() -> DynamicImage {
    DynamicImage::ImageRgb8(ImageBuffer::from_fn(64, 32, |x, y| {
        Rgb([(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8])
    }))
}

#[test]
fn path_and_decoded_inputs_produce_one_representation() {
    let image = test_image();
    let path =
        std::env::temp_dir().join(format!("hipfire-prepared-image-{}.png", std::process::id()));
    image.save(&path).expect("write test image");

    let from_path = prepare_image(&path, 16, 2, 2).expect("prepare path input");
    let from_memory = prepare_decoded_image(&image, 16, 2, 2).expect("prepare decoded input");
    assert_eq!(from_path, from_memory);
    assert_eq!(
        from_path.patch_count(),
        from_path.grid_height() * from_path.grid_width()
    );
    assert_eq!(from_path.grid_height() % 2, 0);
    assert_eq!(from_path.grid_width() % 2, 0);
    assert_eq!(
        from_path.visual_tokens(),
        (from_path.grid_height() / 2) * (from_path.grid_width() / 2)
    );
    assert_eq!(
        from_path.patches().len(),
        from_path.patch_count() * 2 * 3 * 16 * 16
    );
}

#[test]
fn invalid_model_geometry_is_fallible() {
    let image = test_image();
    assert!(matches!(
        prepare_decoded_image(&image, 0, 2, 2),
        Err(ImageInputError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        prepare_decoded_image(&image, 16, 0, 2),
        Err(ImageInputError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        prepare_decoded_image(&image, 16, 2, 0),
        Err(ImageInputError::InvalidConfiguration(_))
    ));
}

#[test]
fn malformed_chw_and_partial_patches_are_rejected() {
    assert!(matches!(
        extract_patches(&vec![0.0; 47], 3, 4, 4, 2, 1),
        Err(ImageInputError::LayoutMismatch {
            expected: 48,
            actual: 47
        })
    ));
    assert!(matches!(
        extract_patches(&vec![0.0; 3 * 5 * 4], 3, 5, 4, 2, 1),
        Err(ImageInputError::InvalidDimensions(_))
    ));
}

#[test]
fn smart_resize_rejects_zero_and_preserves_alignment() {
    assert!(smart_resize(0, 32, 32, 3136, 1_003_520).is_err());
    assert!(smart_resize(32, 32, 0, 3136, 1_003_520).is_err());
    assert!(smart_resize(1, 1000, 32, 3136, 1_003_520).is_err());
    let (height, width) = smart_resize(123, 456, 32, 3136, 1_003_520).expect("valid smart resize");
    assert_eq!(height % 32, 0);
    assert_eq!(width % 32, 0);
}
