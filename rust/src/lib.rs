//! # Framed Message Protocol (FMP) — Rust Reference Implementation
//!
//! Lightweight, extensible binary framing for P2P and mesh networks.
//!
//! Designed for **NovaNet / xMesh / QNET** and similar decentralized systems.
//!
//! ## Features
//! - Fixed 8-byte header (Version + Flags + Type + Reserved + u32 Length)
//! - Typed messages (DATA, PING, PONG, CLOSE, ERROR, HANDSHAKE)
//! - Optional CRC32 checksum
//! - Async `FramedStream` generic over any `AsyncRead + AsyncWrite` (Tokio, Quinn, etc.)
//! - Strong DoS protection via configurable max frame size
//! - Forward compatible design
//!
//! ## Example
//! ```ignore
//! use framed_message_protocol::{FramedStream, Frame};
//! use tokio::net::TcpStream;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let stream = TcpStream::connect("127.0.0.1:9000").await?;
//!     let (r, w) = stream.into_split();
//!     let mut framed = FramedStream::new(r, w);
//!
//!     framed.send(Frame::data(b"Hello P2P mesh!".as_slice())).await?;
//!     let reply = framed.receive().await?;
//!     println!("Received: {:?}", reply);
//!     Ok(())
//! }
//!

pub mod error;
pub mod frame;
pub mod framed;

pub use error::{FmpError, Result};
pub use frame::{Frame, FrameType, flags, VERSION, DEFAULT_MAX_FRAME_SIZE};
pub use framed::{FramedStream, FramedRead, FramedWrite};