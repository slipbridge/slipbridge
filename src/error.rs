use std::fmt;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SlipbridgeError>;

#[derive(Debug, Error)]
pub enum SlipbridgeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unsupported: {0}")]
    Unsupported(String),

    #[error("Transport error: {0}")]
    Transport(String),
}

impl SlipbridgeError {
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(message.into())
    }

    pub fn transport(message: impl fmt::Display) -> Self {
        Self::Transport(message.to_string())
    }
}
