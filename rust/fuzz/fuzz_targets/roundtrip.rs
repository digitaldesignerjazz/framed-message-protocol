#![no_main]

use libfuzzer_sys::fuzz_target;
use bytes::BytesMut;
use framed_message_protocol::{Frame, FrameType};

/// Structure-aware roundtrip fuzzing.
/// We generate somewhat valid frames and check that encode -> decode is stable.
/// This catches serialization/deserialization mismatches and length/flag bugs.
fuzz_target!(|data: &[u8]| {
    // Try to interpret input as a somewhat plausible frame
    if data.len() < 8 {
        return;
    }

    let version = data[0];
    let flags = data[1];
    let frame_type = data[2];
    let length = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    // Only fuzz reasonable sizes to keep fuzzer fast
    if length > 64 * 1024 {
        return;
    }

    let payload_len = std::cmp::min(length as usize, data.len().saturating_sub(8));
    let payload = &data[8..8 + payload_len];

    let frame = Frame {
        version,
        flags,
        frame_type: FrameType::from(frame_type),
        length,
        payload: bytes::Bytes::copy_from_slice(payload),
    };

    let encoded = frame.encode();
    let mut buf = BytesMut::from(&encoded[..]);
    if let Ok(decoded) = Frame::decode(&mut buf) {
        // Basic invariant checks
        assert_eq!(decoded.version, version);
        assert_eq!(decoded.length as usize, decoded.payload.len());
    }
});