package fmp

import (
	"testing"
)

// FuzzDecodeRaw tests the Decode function with arbitrary byte input.
// This is the Go equivalent of the Rust decode_raw cargo-fuzz target.
func FuzzDecodeRaw(f *testing.F) {
	// Seed corpus with interesting cases
	f.Add([]byte{0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00})                         // minimal valid DATA
	f.Add([]byte{0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0, 0, 0, 0, 0, 0, 0, 1}) // PING with timestamp
	f.Add([]byte{0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10})                         // truncated
	f.Add([]byte{0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00})                         // wrong version
	f.Add([]byte{0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 1, 2, 3, 4, 5})         // checksum flag but short payload

	f.Fuzz(func(t *testing.T, data []byte) {
		frame, _, err := Decode(data)
		if err != nil {
			// Errors are expected for invalid input — we mainly care about no panics
			return
		}
		// If decode succeeded, basic invariants should hold
		if int(frame.Length) != len(frame.Payload) {
			t.Errorf("length mismatch: declared %d, actual payload %d", frame.Length, len(frame.Payload))
		}
		if frame.Version != Version {
			t.Errorf("unexpected version after successful decode: %d", frame.Version)
		}
	})
}

// FuzzRoundtrip tests encode -> decode roundtrips with generated frames.
func FuzzRoundtrip(f *testing.F) {
	// Seed with some valid frames
	f.Add(uint8(TypeData), uint8(0), []byte("hello"))
	f.Add(uint8(TypePing), uint8(0), []byte{0, 0, 0, 0, 0, 0, 0, 42})
	f.Add(uint8(TypeClose), uint8(0), []byte{0, 42, 'g', 'o', 'o', 'd', 'b', 'y', 'e'})

	f.Fuzz(func(t *testing.T, ftype uint8, flags uint8, payload []byte) {
		if len(payload) > 64*1024 {
			return // keep it reasonable
		}

		frame := Frame{
			Version: Version,
			Flags:   flags,
			Type:    FrameType(ftype),
			Length:  uint32(len(payload)),
			Payload: payload,
		}

		encoded := frame.Encode()
		decoded, consumed, err := Decode(encoded)
		if err != nil {
			t.Fatalf("decode after encode failed: %v", err)
		}

		if consumed != len(encoded) {
			t.Errorf("consumed bytes mismatch: got %d want %d", consumed, len(encoded))
		}

		// Invariants
		if decoded.Version != Version {
			t.Error("version changed after roundtrip")
		}
		if int(decoded.Length) != len(decoded.Payload) {
			t.Error("length invariant violated after roundtrip")
		}
		if decoded.Type != frame.Type {
			t.Error("type changed after roundtrip")
		}
	})
}

// FuzzDecodeChecksum specifically targets checksum edge cases.
func FuzzDecodeChecksum(f *testing.F) {
	f.Add([]byte{0x01, FlagHasChecksum, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 1, 2, 3, 4}) // valid length but no real CRC
	f.Add([]byte{0x01, FlagHasChecksum, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00})             // checksum flag + zero length

	f.Fuzz(func(t *testing.T, data []byte) {
		_, _, err := Decode(data)
		// We don't assert on error — we just want to ensure no crash/panic
		_ = err
	})
}