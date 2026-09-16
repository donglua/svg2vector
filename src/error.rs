use thiserror::Error;

/// A conversion error. Unsupported features are rejected rather than omitted.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid SVG XML: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("invalid SVG: {0}")]
    Invalid(String),
    #[error("unsupported SVG feature: {0}")]
    Unsupported(String),
}
