#![no_main]

use libfuzzer_sys::fuzz_target;
use bytes::BytesMut;
use framed_message_protocol::Frame;

/// Fuzzes the raw byte decoding path.
/// This is the most important target for finding parser crashes,
/// DoS vectors (huge length), and logic bugs in header handling.
fuzz_target!(|data: &[u8]| {
    let mut buf = BytesMut::from(data);
    // We ignore the result on purpose — we want to see if it panics or OOMs
    let _ = Frame::decode(&mut buf);
});