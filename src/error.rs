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

impl From<wreq::Error> for SpiderError {
    fn from(err: wreq::Error) -> Self {
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

// ─── Web API Error (Task 7) ────────────────────────────────────────────────

use axum::{http::StatusCode, response::{IntoResponse, Json, Response}};
use serde_json::{json, Value};
use thiserror::Error;

/// Web API 错误类型，支持 `?` 操作符自动转换 `anyhow::Error` / `rusqlite::Error`
#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        // 保持与 ApiResponse::err 一致的格式：{code: -1, msg: ...}
        let body: Value = json!({ "code": -1, "msg": self.to_string() });
        (status, Json(body)).into_response()
    }
}

/// Web API Result 类型别名
pub type AppResult<T> = std::result::Result<T, AppError>;