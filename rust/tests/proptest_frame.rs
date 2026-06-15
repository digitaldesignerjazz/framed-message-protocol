//! Property-based tests (proptest) for Frame encoding/decoding.
//! These act as a fast, deterministic fuzzing layer that runs with `cargo test`.

use bytes::BytesMut;
use proptest::prelude::*;
use framed_message_protocol::{Frame, FrameType, flags, VERSION};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Roundtrip property: encode(decode(encode(frame))) == original frame
    #[test]
    fn prop_roundtrip_data(frame in any::<Vec<u8>>().prop_map(Frame::data)) {
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::decode(&mut buf).expect("decode should succeed for valid DATA frame");
        prop_assert_eq!(decoded, frame);
    }

    /// Length invariant must always hold after successful decode
    #[test]
    fn prop_length_invariant(payload in any::<Vec<u8>>()) {
        let frame = Frame::data(payload);
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = Frame::decode(&mut buf).unwrap();
        prop_assert_eq!(decoded.length as usize, decoded.payload.len());
    }

    /// Version must be checked strictly
    #[test]
    fn prop_version_check(bad_version in 0u8..=255u8) {
        prop_assume!(bad_version != VERSION);
        let mut header = [bad_version, 0, FrameType::Data as u8, 0, 0, 0, 0, 0];
        let mut buf = BytesMut::from(&header[..]);
        let result = Frame::decode(&mut buf);
        prop_assert!(result.is_err());
    }

    /// Checksum flag with too short payload must fail
    #[test]
    fn prop_checksum_too_short(short_payload in prop::collection::vec(any::<u8>(), 0..3)) {
        let mut frame = Frame::data(short_payload);
        frame.flags |= flags::HAS_CHECKSUM;
        let encoded = frame.encode();
        let mut buf = BytesMut::from(&encoded[..]);
        let result = Frame::decode(&mut buf);
        prop_assert!(matches!(result, Err(_)));
    }

    /// Very large length in header should be rejected by decode if > reasonable size
    /// (we test the decode logic itself; higher layer enforces max_frame_size)
    #[test]
    fn prop_large_length_rejected(length in (16*1024*1024u32..=u32::MAX)) {
        // Construct a header with huge length but no payload
        let mut header = BytesMut::with_capacity(8);
        header.extend_from_slice(&[
            VERSION,
            0,
            FrameType::Data as u8,
            0,
            (length >> 24) as u8,
            (length >> 16) as u8,
            (length >> 8) as u8,
            length as u8,
        ]);
        let mut buf = header;
        let result = Frame::decode(&mut buf);
        // Should fail because we don't have the payload bytes
        prop_assert!(result.is_err());
    }
}