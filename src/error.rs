use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::string::FromUtf8Error;

/// Custom error types for the spider
#[derive(Debug)]
pub enum SpiderError {
    /// Error when requesting data from the server
    RequestError(String),
    /// Error when parsing HTML content
    HtmlParseError(String),
    /// Error when decoding content
    DecodingError(String),
    /// Error when chapters are not found
    NotFoundChapters(String),
    /// Error when bypassing Cloudflare protection
    CloudflareBypassError(String),
    /// Error when processing files
    FileError(String),
    /// Error when handling concurrent tasks
    ConcurrencyError(String),
    /// Generic error for other cases
    Other(String),
}

impl Display for SpiderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SpiderError::RequestError(msg) => write!(f, "Request error: {}", msg),
            SpiderError::HtmlParseError(msg) => write!(f, "HTML parse error: {}", msg),
            SpiderError::DecodingError(msg) => write!(f, "Decoding error: {}", msg),
            SpiderError::NotFoundChapters(msg) => write!(f, "Chapters not found: {}", msg),
            SpiderError::CloudflareBypassError(msg) => write!(f, "Cloudflare bypass error: {}", msg),
            SpiderError::FileError(msg) => write!(f, "File error: {}", msg),
            SpiderError::ConcurrencyError(msg) => write!(f, "Concurrency error: {}", msg),
            SpiderError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl StdError for SpiderError {}

impl From<reqwest::Error> for SpiderError {
    fn from(err: reqwest::Error) -> Self {
        SpiderError::RequestError(err.to_string())
    }
}

impl From<std::io::Error> for SpiderError {
    fn from(err: std::io::Error) -> Self {
        SpiderError::FileError(err.to_string())
    }
}

impl From<FromUtf8Error> for SpiderError {
    fn from(err: FromUtf8Error) -> Self {
        SpiderError::DecodingError(err.to_string())
    }
}

impl From<serde_json::Error> for SpiderError {
    fn from(err: serde_json::Error) -> Self {
        SpiderError::DecodingError(err.to_string())
    }
}

/// Result type alias for SpiderError
pub type Result<T> = std::result::Result<T, SpiderError>;