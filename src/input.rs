use std::path::Path;

use image::{DynamicImage, ImageDecoder, ImageReader, metadata::Orientation};

use crate::{
    cli::InputTypeArg,
    error::{Result, SlipbridgeError},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InputKind {
    Image,
    Markdown,
    Text,
}

impl InputKind {
    pub fn as_str(self) -> &'static str {
        match self {
            InputKind::Image => "image",
            InputKind::Markdown => "markdown",
            InputKind::Text => "text",
        }
    }
}

pub fn resolve_input_kind(path: &Path, forced: Option<InputTypeArg>) -> Result<InputKind> {
    if let Some(forced) = forced {
        return Ok(match forced {
            InputTypeArg::Image => InputKind::Image,
            InputTypeArg::Markdown => InputKind::Markdown,
            InputTypeArg::Text => InputKind::Text,
        });
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("png" | "jpg" | "jpeg" | "webp" | "gif") => Ok(InputKind::Image),
        Some("md") => Ok(InputKind::Markdown),
        Some("txt") => Ok(InputKind::Text),
        _ => Err(SlipbridgeError::invalid_argument(format!(
            "unable to detect input type for '{}'; use --type",
            path.display()
        ))),
    }
}

pub fn load_oriented_image(path: &Path) -> Result<DynamicImage> {
    let reader = ImageReader::open(path)?;
    let reader = reader.with_guessed_format().map_err(|err| {
        SlipbridgeError::invalid_argument(format!(
            "failed to determine image format for '{}': {err}",
            path.display()
        ))
    })?;

    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);

    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);

    Ok(image)
}
