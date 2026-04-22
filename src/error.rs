use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocsiteError {
    #[error("request failed for {url}: {message}")]
    Request { url: String, message: String },

    #[error("unexpected status {status} for {url}")]
    HttpStatus { url: String, status: u16 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("site detection failed for {0}")]
    DetectionFailed(String),

    #[error("browser fallback requested but unavailable: {0}")]
    BrowserUnavailable(String),
}

pub type Result<T> = std::result::Result<T, DocsiteError>;
