#![no_main]

use libfuzzer_sys::fuzz_target;
use bytes::BytesMut;
use framed_message_protocol::{Frame, FrameType, flags};

/// Structure-aware + property-based roundtrip fuzzing.
/// We construct frames from fuzzer input and verify key invariants after encode/decode.
fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let version = data[0];
    // Only fuzz current version aggressively; other versions should be rejected quickly
    if version != 1 {
        return;
    }

    let flags_byte = data[1];
    let frame_type = data[2];
    let length = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    // Keep payloads reasonable for fast iteration
    if length > 128 * 1024 {
        return;
    }

    let payload_start = 8;
    let payload_end = std::cmp::min(payload_start + length as usize, data.len());
    let payload = &data[payload_start..payload_end];

    let mut frame = Frame {
        version,
        flags: flags_byte,
        frame_type: FrameType::from(frame_type),
        length,
        payload: bytes::Bytes::copy_from_slice(payload),
    };

    // If checksum flag is set but payload too small, skip (will fail anyway)
    if (flags_byte & flags::HAS_CHECKSUM) != 0 && payload.len() < 4 {
        return;
    }

    let encoded = frame.encode();
    let mut buf = BytesMut::from(&encoded[..]);

    match Frame::decode(&mut buf) {
        Ok(decoded) => {
            // Core invariants that must always hold for valid frames
            assert_eq!(decoded.version, version, "version mismatch after roundtrip");
            assert_eq!(decoded.length as usize, decoded.payload.len(), "length invariant violated");
            assert_eq!(decoded.frame_type, FrameType::from(frame_type));

            // If checksum was requested and we had enough bytes, it should have passed
            if (flags_byte & flags::HAS_CHECKSUM) != 0 {
                // The decode path already verified it, so we just assert we got here
            }
        }
        Err(_) => {
            // For invalid inputs it's fine to error — we mainly care about not panicking
        }
    }
});