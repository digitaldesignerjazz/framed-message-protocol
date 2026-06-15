package fmp

import "io"

// FramedConn wraps an io.ReadWriter and provides framed message I/O.
type FramedConn struct {
	conn io.ReadWriter
}

// NewFramedConn creates a new FramedConn.
func NewFramedConn(conn io.ReadWriter) *FramedConn {
	return &FramedConn{conn: conn}
}

// Send writes a frame to the underlying connection.
func (f *FramedConn) Send(frame Frame) error {
	_, err := f.conn.Write(frame.Encode())
	return err
}

// Receive reads exactly one frame from the connection.
func (f *FramedConn) Receive() (Frame, error) {
	return DecodeFromReader(f.conn)
}

// Close closes the underlying connection if it implements io.Closer.
func (f *FramedConn) Close() error {
	if closer, ok := f.conn.(io.Closer); ok {
		return closer.Close()
	}
	return nil
}