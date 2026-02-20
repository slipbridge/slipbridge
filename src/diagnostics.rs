use image::{DynamicImage, ImageBuffer, Rgba};
use rusb::UsbContext;
use serde::Serialize;

use crate::{
    error::Result,
    render::{RenderOptions, render_image_for_thermal},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn is_ok(&self) -> bool {
        self.checks
            .iter()
            .all(|check| !matches!(check.status, CheckStatus::Fail))
    }
}

pub fn run_doctor() -> Result<DoctorReport> {
    let mut checks = Vec::new();

    match rusb::Context::new() {
        Ok(context) => match context.devices() {
            Ok(devices) => {
                let status = if devices.is_empty() {
                    CheckStatus::Warn
                } else {
                    CheckStatus::Pass
                };

                checks.push(DoctorCheck {
                    name: "usb_access".to_owned(),
                    status,
                    detail: format!("enumerated {} USB devices", devices.len()),
                })
            }
            Err(err) => checks.push(DoctorCheck {
                name: "usb_access".to_owned(),
                status: CheckStatus::Fail,
                detail: format!("failed to enumerate USB devices: {err}"),
            }),
        },
        Err(err) => checks.push(DoctorCheck {
            name: "usb_access".to_owned(),
            status: CheckStatus::Fail,
            detail: format!("failed to initialize USB context: {err}"),
        }),
    }

    let sample = ImageBuffer::from_fn(16, 16, |x, y| {
        if (x + y) % 2 == 0 {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([255, 255, 255, 255])
        }
    });

    match render_image_for_thermal(
        DynamicImage::ImageRgba8(sample),
        RenderOptions { max_width: 384 },
    ) {
        Ok(rendered) => checks.push(DoctorCheck {
            name: "image_pipeline".to_owned(),
            status: CheckStatus::Pass,
            detail: format!(
                "rendered {}x{} image into {} bytes",
                rendered.width_px,
                rendered.height_px,
                rendered.raster_data.len()
            ),
        }),
        Err(err) => checks.push(DoctorCheck {
            name: "image_pipeline".to_owned(),
            status: CheckStatus::Fail,
            detail: format!("render pipeline failed: {err}"),
        }),
    }

    checks.push(DoctorCheck {
        name: "transport_tcp".to_owned(),
        status: CheckStatus::Warn,
        detail: "tcp transport support compiled in (connectivity not tested)".to_owned(),
    });

    Ok(DoctorReport { checks })
}
