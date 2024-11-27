use crate::error::SpiderError::{OtherError, RequestError};
use pyo3::PyErr;
use std::fmt::Display;
use std::io::Error;


#[derive(Debug)]
pub enum SpiderError {
    RequestError(reqwest::Error),
    
    DecodingError,
    
    NotFoundChapters,
    
    HtmlParseError,
    
    OtherError(std::io::Error),
    
    UnknownError,
    
}

impl From<std::io::Error> for SpiderError {
    fn from(value: Error) -> Self {
        OtherError(value)
    }
}

impl From<reqwest::Error> for SpiderError {
    fn from(value: reqwest::Error) -> Self {
        RequestError(value)
    }
}

impl From<PyErr> for SpiderError {
    fn from(value: PyErr) -> Self {
        OtherError(value.into())
    }
}