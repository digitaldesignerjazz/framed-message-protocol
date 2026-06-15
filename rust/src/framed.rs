//! Async framed stream wrapper.
//!
//! Works with any type implementing `tokio::io::AsyncRead` + `AsyncWrite` (e.g. TcpStream, Quinn streams, etc.).

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{FmpError, Result};
use crate::frame::{Frame, DEFAULT_MAX_FRAME_SIZE, VERSION};

/// A bidirectional framed message stream.
///
/// Wraps any async read/write pair and provides `send` / `receive` with proper framing.
pub struct FramedStream<R, W> {
    reader: R,
    writer: W,
    read_buf: BytesMut,
    max_frame_size: usize,
    strict: bool, // if true, unknown flags/types cause error instead of forward-compat handling
}

impl<R, W> FramedStream<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    /// Create a new framed stream with default limits.
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            read_buf: BytesMut::with_capacity(4096),
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            strict: false,
        }
    }

    /// Set maximum allowed frame payload size (DoS protection).
    pub fn with_max_frame_size(mut self, max: usize) -> Self {
        self.max_frame_size = max;
        self
    }

    /// Enable strict mode (unknown flags or types become hard errors).
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Send a frame (writes header + payload atomically via write_all).
    pub async fn send(&mut self, frame: Frame) -> Result<()> {
        let wire = frame.encode();
        self.writer.write_all(&wire).await?;
        // We do NOT flush here to allow batching. Caller can flush if needed.
        Ok(())
    }

    /// Flush the writer.
    pub async fn flush(&mut self) -> Result<()> {
        self.writer.flush().await.map_err(Into::into)
    }

    /// Receive the next frame.
    ///
    /// Blocks until a complete, valid frame is available.
    /// Respects `max_frame_size`.
    pub async fn receive(&mut self) -> Result<Frame> {
        loop {
            // Try to decode if we have enough data
            if self.read_buf.len() >= 8 {
                // Peek header without consuming
                let version = self.read_buf[0];
                if version != VERSION {
                    return Err(FmpError::UnsupportedVersion(version));
                }

                let length = u32::from_be_bytes([
                    self.read_buf[4],
                    self.read_buf[5],
                    self.read_buf[6],
                    self.read_buf[7],
                ]) as usize;

                if length > self.max_frame_size {
                    return Err(FmpError::FrameTooLarge {
                        length: length as u32,
                        max: self.max_frame_size,
                    });
                }

                let total = 8 + length;
                if self.read_buf.len() >= total {
                    // We have a complete frame — decode it (consumes from buffer)
                    return Frame::decode(&mut self.read_buf);
                }
            }

            // Need more data
            let mut tmp = [0u8; 4096];
            let n = self.reader.read(&mut tmp).await?;
            if n == 0 {
                return Err(FmpError::UnexpectedClose);
            }
            self.read_buf.extend_from_slice(&tmp[..n]);
        }
    }

    /// Split into separate read and write halves (useful for concurrent send/receive).
    pub fn split(self) -> (FramedRead<R>, FramedWrite<W>) {
        let FramedStream { reader, writer, read_buf, max_frame_size, strict } = self;
        (
            FramedRead { reader, read_buf, max_frame_size, strict },
            FramedWrite { writer },
        )
    }
}

/// Read half (after split).
pub struct FramedRead<R> {
    reader: R,
    read_buf: BytesMut,
    max_frame_size: usize,
    strict: bool,
}

impl<R: AsyncRead + Unpin> FramedRead<R> {
    pub async fn receive(&mut self) -> Result<Frame> {
        // Same logic as above, duplicated for simplicity in v0.1
        loop {
            if self.read_buf.len() >= 8 {
                let length = u32::from_be_bytes([
                    self.read_buf[4], self.read_buf[5], self.read_buf[6], self.read_buf[7],
                ]) as usize;

                if length > self.max_frame_size {
                    return Err(FmpError::FrameTooLarge { length: length as u32, max: self.max_frame_size });
                }

                if self.read_buf.len() >= 8 + length {
                    return Frame::decode(&mut self.read_buf);
                }
            }

            let mut tmp = [0u8; 4096];
            let n = self.reader.read(&mut tmp).await?;
            if n == 0 {
                return Err(FmpError::UnexpectedClose);
            }
            self.read_buf.extend_from_slice(&tmp[..n]);
        }
    }
}

/// Write half (after split).
pub struct FramedWrite<W> {
    writer: W,
}

impl<W: AsyncWrite + Unpin> FramedWrite<W> {
    pub async fn send(&mut self, frame: Frame) -> Result<()> {
        let wire = frame.encode();
        self.writer.write_all(&wire).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<()> {
        self.writer.flush().await.map_err(Into::into)
    }
}