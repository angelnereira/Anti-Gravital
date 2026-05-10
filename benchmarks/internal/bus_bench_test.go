// Package internal_test contains integration benchmarks for the Anti-Gravital
// Memory Bus. These benchmarks require the Rust Shield to have created the
// shared memory segment; they are skipped automatically when the segment is
// not present.
//
// To run against a live segment:
//  1. Start the Rust side: cargo run -p ag-core --example bus_writer
//  2. In a separate terminal: go test -bench=. -benchmem ./benchmarks/internal/
package internal_test

import (
	"testing"

	"github.com/gravital-labs/anti-gravital/ag-runtime/bus"
)

// openBusOrSkip opens the default bus segment. If the segment does not exist
// (the Rust side is not running) the benchmark is skipped rather than failing.
func openBusOrSkip(b *testing.B) *bus.MemoryBus {
	b.Helper()
	mb, err := bus.Open(bus.BusDefaultName)
	if err != nil {
		b.Skipf("shared memory segment not available (%v); start the Rust Shield first", err)
	}
	return mb
}

// BenchmarkClaimReadSlot measures how quickly the Go side can scan the ring
// buffer and find READY slots. In a production scenario the throughput here
// is bounded by the Rust write rate and the number of goroutine workers.
func BenchmarkClaimReadSlot(b *testing.B) {
	mb := openBusOrSkip(b)
	defer mb.Close()

	b.ResetTimer()
	b.ReportAllocs()

	var found int
	for i := 0; i < b.N; i++ {
		idx := mb.ClaimReadSlot()
		if idx >= 0 {
			found++
			// Immediately respond so the Rust side can recycle the slot.
			mb.WriteResponse(idx, 200, []byte(`"ok"`))
		}
	}

	b.ReportMetric(float64(found)/float64(b.N)*100, "%_hit_rate")
}

// BenchmarkReadRequest measures the cost of decoding a fully-populated request
// slot on the Go side. This benchmark runs without a live Rust segment to
// isolate the decoding cost from bus latency.
func BenchmarkReadRequest(b *testing.B) {
	mb := openBusOrSkip(b)
	defer mb.Close()

	b.ResetTimer()
	b.ReportAllocs()

	for i := 0; i < b.N; i++ {
		idx := mb.ClaimReadSlot()
		if idx >= 0 {
			req := mb.ReadRequest(idx)
			mb.WriteResponse(idx, 200, req.Payload)
		}
	}
}
