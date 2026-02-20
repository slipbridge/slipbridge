use image::{
    DynamicImage, GrayImage, RgbaImage,
    imageops::{self, BiLevel, FilterType},
};

use crate::error::{Result, SlipbridgeError};

#[derive(Copy, Clone, Debug)]
pub struct RenderOptions {
    pub max_width: u32,
}

#[derive(Clone, Debug)]
pub struct RenderedImage {
    pub width_px: u32,
    pub height_px: u32,
    pub bytes_per_row: usize,
    pub raster_data: Vec<u8>,
}

pub fn render_image_for_thermal(
    image: DynamicImage,
    options: RenderOptions,
) -> Result<RenderedImage> {
    if options.max_width == 0 {
        return Err(SlipbridgeError::invalid_argument(
            "render max width must be > 0",
        ));
    }

    let flattened = flatten_alpha(image);
    let (source_width, source_height) = flattened.dimensions();

    if source_width == 0 || source_height == 0 {
        return Err(SlipbridgeError::invalid_argument(
            "image has zero dimensions",
        ));
    }

    let target_width = source_width.min(options.max_width);
    let target_height = if target_width == source_width {
        source_height
    } else {
        ((source_height as f64 * target_width as f64 / source_width as f64)
            .round()
            .max(1.0)) as u32
    };

    let resized = if target_width == source_width && target_height == source_height {
        flattened
    } else {
        imageops::resize(
            &flattened,
            target_width,
            target_height,
            FilterType::Triangle,
        )
    };

    let mut grayscale: GrayImage = DynamicImage::ImageRgba8(resized).into_luma8();

    if grayscale.width() == 0 || grayscale.height() == 0 {
        return Err(SlipbridgeError::invalid_argument(
            "image became empty after resize",
        ));
    }

    imageops::dither(&mut grayscale, &BiLevel);

    let bytes_per_row = usize::try_from(grayscale.width().div_ceil(8)).map_err(|_| {
        SlipbridgeError::invalid_argument("image width is too large for this platform")
    })?;

    let mut raster_data = Vec::with_capacity(
        bytes_per_row
            .checked_mul(usize::try_from(grayscale.height()).unwrap_or(usize::MAX))
            .ok_or_else(|| SlipbridgeError::invalid_argument("render buffer too large"))?,
    );

    for y in 0..grayscale.height() {
        for byte_idx in 0..bytes_per_row {
            let mut packed = 0u8;
            let base_x = byte_idx * 8;

            for bit in 0..8usize {
                let x = base_x + bit;
                if x >= grayscale.width() as usize {
                    continue;
                }

                let pixel = grayscale.get_pixel(x as u32, y).0[0];
                if pixel < 128 {
                    packed |= 1 << (7 - bit);
                }
            }

            raster_data.push(packed);
        }
    }

    Ok(RenderedImage {
        width_px: grayscale.width(),
        height_px: grayscale.height(),
        bytes_per_row,
        raster_data,
    })
}

fn flatten_alpha(image: DynamicImage) -> RgbaImage {
    let mut rgba = image.into_rgba8();

    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel.0[3]);
        if alpha == 255 {
            continue;
        }

        let inv_alpha = 255u16 - alpha;
        for channel in 0..3 {
            let src = u16::from(pixel.0[channel]);
            let blended = (src * alpha + 255 * inv_alpha) / 255;
            pixel.0[channel] = blended as u8;
        }

        pixel.0[3] = 255;
    }

    rgba
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, ImageBuffer, Rgba};

    use super::{RenderOptions, render_image_for_thermal};

    #[test]
    fn packs_rows_into_expected_bytes() {
        let image = ImageBuffer::from_fn(8, 1, |x, _| {
            if x < 4 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });

        let rendered = render_image_for_thermal(
            DynamicImage::ImageRgba8(image),
            RenderOptions { max_width: 8 },
        )
        .expect("render should succeed");

        assert_eq!(rendered.bytes_per_row, 1);
        assert_eq!(rendered.raster_data, vec![0b1111_0000]);
    }
}
