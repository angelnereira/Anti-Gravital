package bus

import (
	"context"
	"runtime"
	"sync"
)

// SlotContext provides a handler with access to the decoded request and a
// mechanism to send a response back through the ring buffer.
type SlotContext struct {
	RequestID uint64
	Method    uint8
	Path      []byte
	Auth      []byte
	Body      []byte

	slotIdx int
	bus     *MemoryBus
	once    sync.Once
}

// Respond writes the given status and body back into the ring buffer slot.
// Safe to call exactly once per SlotContext; subsequent calls are no-ops.
func (s *SlotContext) Respond(status uint16, body []byte) {
	s.once.Do(func() {
		s.bus.WriteResponse(s.slotIdx, status, body)
	})
}

// RequestHandler is a function that processes a single request from the bus.
// The implementation must call ctx.Respond exactly once before returning.
type RequestHandler func(ctx *SlotContext)

// BusReader manages a pool of goroutines that continuously drain READY slots
// from the ring buffer and dispatch them to a RequestHandler.
type BusReader struct {
	bus      *MemoryBus
	handler  RequestHandler
	workers  int
	shutdown chan struct{}
	wg       sync.WaitGroup
}

// NewBusReader creates a BusReader that will use `workers` goroutines to
// process requests. A value of 0 defaults to runtime.NumCPU().
func NewBusReader(bus *MemoryBus, handler RequestHandler, workers int) *BusReader {
	if workers <= 0 {
		workers = runtime.NumCPU()
	}
	return &BusReader{
		bus:      bus,
		handler:  handler,
		workers:  workers,
		shutdown: make(chan struct{}),
	}
}

// Start launches the worker goroutines. It returns immediately; call Stop to
// signal shutdown and Wait to block until all workers exit.
func (r *BusReader) Start(ctx context.Context) {
	for i := 0; i < r.workers; i++ {
		r.wg.Add(1)
		go r.work(ctx)
	}
}

// Stop signals all worker goroutines to exit after finishing their current
// slot. Call Wait to block until shutdown is complete.
func (r *BusReader) Stop() {
	close(r.shutdown)
}

// Wait blocks until all workers have exited.
func (r *BusReader) Wait() {
	r.wg.Wait()
}

func (r *BusReader) work(ctx context.Context) {
	defer r.wg.Done()

	for {
		select {
		case <-ctx.Done():
			return
		case <-r.shutdown:
			return
		default:
		}

		idx := r.bus.ClaimReadSlot()
		if idx < 0 {
			// No READY slot; yield to avoid burning the CPU when idle.
			runtime.Gosched()
			continue
		}

		req := r.bus.ReadRequest(idx)
		slot := &SlotContext{
			RequestID: req.RequestID,
			Method:    req.Method,
			Path:      req.Path,
			Auth:      req.AuthClaims,
			Body:      req.Payload,
			slotIdx:   idx,
			bus:       r.bus,
		}

		// Invoke the handler. If it panics, recover and send a 500 to unblock
		// the Rust Shield; a panicking handler must not stall the bus.
		func() {
			defer func() {
				if p := recover(); p != nil {
					slot.Respond(500, []byte(`{"error":"internal_server_error"}`))
				}
			}()
			r.handler(slot)
			// Ensure a response is always sent even if the handler forgot.
			slot.Respond(500, []byte(`{"error":"handler_did_not_respond"}`))
		}()
	}
}
