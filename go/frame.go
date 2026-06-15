package fmp

import (
	"encoding/binary"
	"hash/crc32"
	"io"
)

// Version is the current protocol version.
const Version uint8 = 0x01

// MaxFrameSize is the recommended maximum payload size.
const MaxFrameSize = 16 * 1024 * 1024

// Flags
const (
	FlagHasChecksum   uint8 = 0x01
	FlagCompressed    uint8 = 0x02
	FlagHighPriority  uint8 = 0x04
	FlagHasExtensions uint8 = 0x08
)

// FrameType represents the semantic type of a frame.
type FrameType uint8

const (
	TypeData      FrameType = 0x00
	TypePing      FrameType = 0x01
	TypePong      FrameType = 0x02
	TypeClose     FrameType = 0x03
	TypeError     FrameType = 0x04
	TypeHandshake FrameType = 0x05
	TypeAck       FrameType = 0x06
)

// Frame represents a single framed message.
type Frame struct {
	Version   uint8
	Flags     uint8
	Type      FrameType
	Length    uint32
	Payload   []byte
}

// NewData creates a new DATA frame.
func NewData(payload []byte) Frame {
	return Frame{
		Version: Version,
		Type:    TypeData,
		Length:  uint32(len(payload)),
		Payload: payload,
	}
}

// NewPing creates a PING frame (optional 8-byte timestamp).
func NewPing(timestampMs *uint64) Frame {
	var payload []byte
	if timestampMs != nil {
		payload = make([]byte, 8)
		binary.BigEndian.PutUint64(payload, *timestampMs)
	}
	return Frame{
		Version: Version,
		Type:    TypePing,
		Length:  uint32(len(payload)),
		Payload: payload,
	}
}

// NewPong creates a PONG frame.
func NewPong(echoPayload []byte) Frame {
	if echoPayload == nil {
		// Use current time as simple timestamp
		ts := uint64(0) // In real code use time.Now().UnixMilli()
		echoPayload = make([]byte, 8)
		binary.BigEndian.PutUint64(echoPayload, ts)
	}
	return Frame{
		Version: Version,
		Type:    TypePong,
		Length:  uint32(len(echoPayload)),
		Payload: echoPayload,
	}
}

// NewClose creates a CLOSE frame.
func NewClose(reasonCode uint16, reason string) Frame {
	payload := make([]byte, 2+len(reason))
	binary.BigEndian.PutUint16(payload[0:2], reasonCode)
	copy(payload[2:], reason)
	return Frame{
		Version: Version,
		Type:    TypeClose,
		Length:  uint32(len(payload)),
		Payload: payload,
	}
}

// NewError creates an ERROR frame.
func NewError(code uint16, message string) Frame {
	payload := make([]byte, 2+len(message))
	binary.BigEndian.PutUint16(payload[0:2], code)
	copy(payload[2:], message)
	return Frame{
		Version: Version,
		Type:    TypeError,
		Length:  uint32(len(payload)),
		Payload: payload,
	}
}

// Encode serializes the frame to wire format.
func (f Frame) Encode() []byte {
	buf := make([]byte, 8+len(f.Payload))
	buf[0] = f.Version
	buf[1] = f.Flags
	buf[2] = uint8(f.Type)
	buf[3] = 0 // reserved
	binary.BigEndian.PutUint32(buf[4:8], f.Length)
	copy(buf[8:], f.Payload)
	return buf
}

// Decode deserializes a frame from a byte slice.
// It returns the frame and the number of bytes consumed, or an error.
func Decode(data []byte) (Frame, int, error) {
	if len(data) < 8 {
		return Frame{}, 0, ErrIncompleteHeader
	}

	version := data[0]
	if version != Version {
		return Frame{}, 0, &ErrUnsupportedVersion{Version: version}
	}

	flags := data[1]
	frameType := FrameType(data[2])
	length := binary.BigEndian.Uint32(data[4:8])

	if int(length) > MaxFrameSize {
		return Frame{}, 0, &ErrFrameTooLarge{Length: length, Max: MaxFrameSize}
	}

	total := 8 + int(length)
	if len(data) < total {
		return Frame{}, 0, ErrIncompleteFrame
	}

	payload := make([]byte, length)
	copy(payload, data[8:total])

	// Verify checksum if flag is set
	if flags&FlagHasChecksum != 0 {
		if len(payload) < 4 {
			return Frame{}, 0, ErrChecksumMismatch
		}
		dataPart := payload[:len(payload)-4]
		receivedCRC := binary.BigEndian.Uint32(payload[len(payload)-4:])
		computed := crc32.ChecksumIEEE(dataPart)
		if receivedCRC != computed {
			return Frame{}, 0, ErrChecksumMismatch
		}
		payload = dataPart // strip CRC for convenience
	}

	return Frame{
		Version: version,
		Flags:   flags,
		Type:    frameType,
		Length:  uint32(len(payload)),
		Payload: payload,
	}, total, nil
}

// DecodeFromReader reads exactly one frame from an io.Reader.
func DecodeFromReader(r io.Reader) (Frame, error) {
	header := make([]byte, 8)
	if _, err := io.ReadFull(r, header); err != nil {
		return Frame{}, err
	}

	length := binary.BigEndian.Uint32(header[4:8])
	if int(length) > MaxFrameSize {
		return Frame{}, &ErrFrameTooLarge{Length: length, Max: MaxFrameSize}
	}

	payload := make([]byte, length)
	if _, err := io.ReadFull(r, payload); err != nil {
		return Frame{}, err
	}

	// Reconstruct full frame for Decode (reuses logic)
	full := make([]byte, 8+len(payload))
	copy(full, header)
	copy(full[8:], payload)

	frame, _, err := Decode(full)
	return frame, err
}