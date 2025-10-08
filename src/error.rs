use crate::{constants::ConfigError, service::ServiceConversionError};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("API request failed: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Failed to convert API response: {0}")]
    Conversion(#[from] ServiceConversionError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Screen clearing failed: {0}")]
    ClearScreen(#[from] clearscreen::Error),
}
