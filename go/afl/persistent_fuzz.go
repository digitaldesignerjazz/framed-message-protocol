//go:build ignore

package main

// persistent_fuzz.go
// Example of an AFL++ persistent mode fuzzer for Framed Message Protocol.
// Compile with: go build -o fuzzer persistent_fuzz.go

import (
	"os"

	fmp "github.com/digitaldesignerjazz/framed-message-protocol/go"
)

func main() {
	// AFL++ persistent mode expects the fuzzer to loop forever
	// reading from stdin.
	for {
		// Read one input from AFL++
		data := make([]byte, 64*1024)
		n, err := os.Stdin.Read(data)
		if err != nil || n == 0 {
			break
		}

		// Try to decode the input
		_, _, _ = fmp.Decode(data[:n])

		// AFL++ expects the process to exit(0) after each iteration in persistent mode
		// For true persistent mode you would use __AFL_LOOP, but this simple
		// version works reasonably well with AFL++.
	}
}