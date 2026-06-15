//! Core Frame definition, wire encoding/decoding, and message type helpers.
//!
//! This module is async-free and can be used in no_std contexts with alloc in the future.

use bytes::{Bytes, BytesMut, Buf, BufMut};
use crc32fast::Hasher;

use crate::error::{FmpError, Result};

/// Current protocol version.
pub const VERSION: u8 = 0x01;

/// Maximum recommended frame size (16 MiB). Implementations should enforce this or lower.
pub const DEFAULT_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

// CRC32 (IEEE) via crc32fast for HAS_CHECKSUM support

/// Frame flags (bitmask in header).
pub mod flags {
    pub const HAS_CHECKSUM: u8 = 0x01;
    pub const COMPRESSED:   u8 = 0x02;
    pub const HIGH_PRIORITY: u8 = 0x04;
    pub const HAS_EXTENSIONS: u8 = 0x08;
}

/// Well-known frame types.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameType {
    Data      = 0x00,
    Ping      = 0x01,
    Pong      = 0x02,
    Close     = 0x03,
    Error     = 0x04,
    Handshake = 0x05,
    Ack       = 0x06,
    // 0x07-0x0F reserved for control extensions
    // 0x10+ application defined
}

impl From<u8> for FrameType {
    fn from(v: u8) -> Self {
        match v {
            0x00 => FrameType::Data,
            0x01 => FrameType::Ping,
            0x02 => FrameType::Pong,
            0x03 => FrameType::Close,
            0x04 => FrameType::Error,
            0x05 => FrameType::Handshake,
            0x06 => FrameType::Ack,
            _ => FrameType::Data, // Unknown treated as Data for forward compat (higher layer decides)
        }
    }
}

impl From<FrameType> for u8 {
    fn from(t: FrameType) -> Self {
        t as u8
    }
}

/// A single framed message on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub version: u8,
    pub flags: u8,
    pub frame_type: FrameType,
    pub length: u32,
    pub payload: Bytes,
}

impl Frame {
    /// Create a new DATA frame.
    pub fn data(payload: impl Into<Bytes>) -> Self {
        let payload = payload.into();
        Self {
            version: VERSION,
            flags: 0,
            frame_type: FrameType::Data,
            length: payload.len() as u32,
            payload,
        }
    }

    /// Create a PING frame (optionally with 8-byte timestamp).
    pub fn ping(timestamp_ms: Option<u64>) -> Self {
        let payload = match timestamp_ms {
            Some(ts) => Bytes::from(ts.to_be_bytes().to_vec()),
            None => Bytes::new(),
        };
        Self {
            version: VERSION,
            flags: 0,
            frame_type: FrameType::Ping,
            length: payload.len() as u32,
            payload,
        }
    }

    /// Create a PONG frame (echo or current timestamp).
    pub fn pong(echo_payload: Option<Bytes>) -> Self {
        let payload = echo_payload.unwrap_or_else(|| {
            // simple current time hint (not cryptographically secure)
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            Bytes::from(ts.to_be_bytes().to_vec())
        });
        Self {
            version: VERSION,
            flags: 0,
            frame_type: FrameType::Pong,
            length: payload.len() as u32,
            payload,
        }
    }

    /// Create a CLOSE frame with optional reason.
    pub fn close(reason_code: u16, reason: impl AsRef<str>) -> Self {
        let reason = reason.as_ref();
        let mut payload = BytesMut::with_capacity(2 + reason.len());
        payload.put_u16(reason_code);
        payload.put_slice(reason.as_bytes());
        let payload = payload.freeze();
        Self {
            version: VERSION,
            flags: 0,
            frame_type: FrameType::Close,
            length: payload.len() as u32,
            payload,
        }
    }

    /// Create an ERROR frame.
    pub fn error(code: u16, message: impl AsRef<str>) -> Self {
        let msg = message.as_ref();
        let mut payload = BytesMut::with_capacity(2 + msg.len());
        payload.put_u16(code);
        payload.put_slice(msg.as_bytes());
        let payload = payload.freeze();
        Self {
            version: VERSION,
            flags: 0,
            frame_type: FrameType::Error,
            length: payload.len() as u32,
            payload,
        }
    }

    /// Encode the frame to wire format (header + payload).
    /// Returns the complete on-wire bytes.
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(8 + self.payload.len());
        buf.put_u8(self.version);
        buf.put_u8(self.flags);
        buf.put_u8(self.frame_type.into());
        buf.put_u8(0); // reserved
        buf.put_u32(self.length);
        buf.put_slice(&self.payload);
        buf.freeze()
    }

    /// Decode a frame from a buffer that contains at least the full frame.
    /// Consumes exactly 8 + length bytes from the buffer.
    pub fn decode(buf: &mut BytesMut) -> Result<Self> {
        if buf.len() < 8 {
            return Err(FmpError::Parse("buffer too short for header".into()));
        }

        let version = buf[0];
        if version != VERSION {
            return Err(FmpError::UnsupportedVersion(version));
        }

        let flags = buf[1];
        let frame_type = FrameType::from(buf[2]);
        let _reserved = buf[3];
        let length = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);

        if (flags & flags::HAS_EXTENSIONS) != 0 {
            // Future: parse TLV extensions here. For v0.1 we treat as error or ignore.
            // For now we keep strict: unknown extension bits → error in higher layer.
        }

        let total_needed = 8 + length as usize;
        if buf.len() < total_needed {
            return Err(FmpError::Parse(format!(
                "incomplete frame: have {} need {}",
                buf.len(),
                total_needed
            )));
        }

        // Advance past header
        buf.advance(8);

        let payload = buf.split_to(length as usize).freeze();

        // Verify checksum if present
        if (flags & flags::HAS_CHECKSUM) != 0 {
            if payload.len() < 4 {
                return Err(FmpError::ChecksumMismatch);
            }
            let (data, crc_bytes) = payload.split_at(payload.len() - 4);
            let received_crc = u32::from_be_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);
            let mut hasher = Hasher::new();
            hasher.update(data);
            let computed = hasher.finalize();
            if received_crc != computed {
                return Err(FmpError::ChecksumMismatch);
            }
            // Note: we return the payload *without* the trailing CRC for convenience
            // In real impl you might want to expose raw or provide helper.
            // For simplicity here we strip it in decode when flag present.
            // Better design: keep full payload, document that last 4 bytes are CRC when flag set.
            // For this reference we strip for ergonomics.
            let clean_payload = Bytes::copy_from_slice(data);
            return Ok(Self {
                version,
                flags,
                frame_type,
                length: clean_payload.len() as u32,
                payload: clean_payload,
            });
        }

        Ok(Self {
            version,
            flags,
            frame_type,
            length,
            payload,
        })
    }
}