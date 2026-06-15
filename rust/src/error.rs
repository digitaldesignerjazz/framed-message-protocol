//! Error types for the Framed Message Protocol.

use thiserror::Error;

/// Main error type for FMP operations.
#[derive(Error, Debug)]
pub enum FmpError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u8),

    #[error("frame too large: {length} bytes (max configured: {max})")]
    FrameTooLarge { length: u32, max: usize },

    #[error("checksum mismatch on frame")]
    ChecksumMismatch,

    #[error("invalid flags or reserved bits set")]
    InvalidFlags,

    #[error("parse error: {0}")]
    Parse(String),

    #[error("protocol violation: {0}")]
    ProtocolViolation(String),

    #[error("connection closed unexpectedly")]
    UnexpectedClose,

    #[error("timeout waiting for frame")]
    Timeout,

    #[error("other error: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, FmpError>;