use crate::{
    error::{Result, SlipbridgeError},
    render::RenderedImage,
};

#[derive(Copy, Clone, Debug)]
pub struct RasterOptions {
    pub max_payload_bytes: usize,
    pub max_rows_per_command: u16,
    pub cut: bool,
}

#[derive(Clone, Debug)]
pub struct EscPosJob {
    pub bytes: Vec<u8>,
    pub chunk_count: usize,
    pub rows_per_chunk: Vec<u16>,
}

pub fn build_raster_print_job(
    rendered: &RenderedImage,
    options: RasterOptions,
) -> Result<EscPosJob> {
    if rendered.bytes_per_row == 0 || rendered.height_px == 0 {
        return Err(SlipbridgeError::invalid_argument(
            "cannot build ESC/POS job for empty image",
        ));
    }

    if rendered.raster_data.len()
        != rendered
            .bytes_per_row
            .checked_mul(rendered.height_px as usize)
            .ok_or_else(|| SlipbridgeError::invalid_argument("image buffer overflow"))?
    {
        return Err(SlipbridgeError::invalid_argument(
            "rendered raster payload length is inconsistent",
        ));
    }

    let bytes_per_row = rendered.bytes_per_row;
    let x = u16::try_from(bytes_per_row).map_err(|_| {
        SlipbridgeError::invalid_argument("image row is too wide for ESC/POS raster mode")
    })?;

    let rows_by_payload = options.max_payload_bytes / bytes_per_row;
    let max_rows = rows_by_payload
        .max(1)
        .min(options.max_rows_per_command.max(1) as usize);

    let mut bytes = Vec::new();
    let mut rows_per_chunk = Vec::new();

    // ESC @ initialize.
    bytes.extend_from_slice(&[0x1B, 0x40]);
    // ESC a 0 left align.
    bytes.extend_from_slice(&[0x1B, 0x61, 0x00]);

    let total_rows = rendered.height_px as usize;
    let mut row_start = 0usize;

    while row_start < total_rows {
        let rows = (total_rows - row_start).min(max_rows);
        let y = u16::try_from(rows)
            .map_err(|_| SlipbridgeError::invalid_argument("chunk row count overflow"))?;

        let start = row_start
            .checked_mul(bytes_per_row)
            .ok_or_else(|| SlipbridgeError::invalid_argument("chunk start overflow"))?;
        let end = start
            .checked_add(rows * bytes_per_row)
            .ok_or_else(|| SlipbridgeError::invalid_argument("chunk end overflow"))?;

        // GS v 0 m xL xH yL yH d1...dk
        bytes.extend_from_slice(&[
            0x1D,
            0x76,
            0x30,
            0x00,
            (x & 0x00FF) as u8,
            (x >> 8) as u8,
            (y & 0x00FF) as u8,
            (y >> 8) as u8,
        ]);
        bytes.extend_from_slice(&rendered.raster_data[start..end]);

        rows_per_chunk.push(y);
        row_start += rows;
    }

    // Feed 3 lines after printing to avoid clipping on some units.
    bytes.extend_from_slice(&[0x1B, 0x64, 0x03]);

    if options.cut {
        // GS V A 0 full cut.
        bytes.extend_from_slice(&[0x1D, 0x56, 0x41, 0x00]);
    }

    Ok(EscPosJob {
        bytes,
        chunk_count: rows_per_chunk.len(),
        rows_per_chunk,
    })
}

#[cfg(test)]
mod tests {
    use super::{RasterOptions, build_raster_print_job};
    use crate::render::RenderedImage;

    #[test]
    fn splits_large_image_into_safe_chunks() {
        let bytes_per_row = 72usize;
        let height_px = 120u32;
        let raster_data = vec![0xAA; bytes_per_row * height_px as usize];

        let rendered = RenderedImage {
            width_px: 576,
            height_px,
            bytes_per_row,
            raster_data,
        };

        let job = build_raster_print_job(
            &rendered,
            RasterOptions {
                max_payload_bytes: 720,
                max_rows_per_command: 255,
                cut: true,
            },
        )
        .expect("job should build");

        // 720 / 72 = 10 rows per chunk -> 12 chunks for 120 rows.
        assert_eq!(job.chunk_count, 12);
        assert!(job.rows_per_chunk.iter().all(|rows| *rows <= 10));
    }
}
