package fmp

import "fmt"

// Sentinel errors
var (
	ErrIncompleteHeader = fmt.Errorf("incomplete header (need 8 bytes)")
	ErrIncompleteFrame  = fmt.Errorf("incomplete frame")
	ErrChecksumMismatch = fmt.Errorf("checksum mismatch")
)

// ErrUnsupportedVersion is returned when the frame version is not supported.
type ErrUnsupportedVersion struct {
	Version uint8
}

func (e *ErrUnsupportedVersion) Error() string {
	return fmt.Sprintf("unsupported protocol version: %d", e.Version)
}

// ErrFrameTooLarge is returned when the declared length exceeds the limit.
type ErrFrameTooLarge struct {
	Length uint32
	Max    int
}

func (e *ErrFrameTooLarge) Error() string {
	return fmt.Sprintf("frame too large: %d bytes (max %d)", e.Length, e.Max)
}