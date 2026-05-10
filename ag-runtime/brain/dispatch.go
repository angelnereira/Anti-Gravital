package brain

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/gravital-labs/anti-gravital/ag-runtime/bus"
)

// Dispatcher bridges the ring buffer BusReader with the application Router.
// It translates raw bus slots into typed Context values, dispatches them to
// the appropriate handler, and writes the response back to the slot.
type Dispatcher struct {
	router *Router
	reader *bus.BusReader
}

// NewDispatcher creates a Dispatcher that uses the given router to dispatch
// requests read from bus. The dispatcher runs workers goroutines.
func NewDispatcher(b *bus.MemoryBus, router *Router, workers int) *Dispatcher {
	d := &Dispatcher{router: router}
	d.reader = bus.NewBusReader(b, d.handle, workers)
	return d
}

// Start launches the worker pool. It returns immediately.
func (d *Dispatcher) Start(ctx context.Context) {
	d.reader.Start(ctx)
}

// Stop signals workers to shut down.
func (d *Dispatcher) Stop() {
	d.reader.Stop()
}

// Wait blocks until all workers have exited.
func (d *Dispatcher) Wait() {
	d.reader.Wait()
}

// handle is the RequestHandler implementation passed to BusReader.
func (d *Dispatcher) handle(slot *bus.SlotContext) {
	method := httpMethodString(slot.Method)
	path := string(slot.Path)

	handler, ok := d.router.Match(method, path)
	if !ok {
		slot.Respond(404, notFoundBody(path))
		return
	}

	// Decode auth claims from the slot's auth_claims field. In Phase 1 these
	// will be verified JWT claims; in Phase 0 they pass through as-is.
	auth := decodeAuth(slot.Auth)

	ctx, rw := newContext(method, path, slot.Body, auth)
	if err := handler(ctx); err != nil {
		if he, ok := err.(*HandlerError); ok {
			slot.Respond(uint16(he.Status), errorBody(he.Code, he.Cause))
		} else {
			slot.Respond(500, errorBody("internal_server_error", err))
		}
		return
	}

	if !rw.written {
		// Handler returned nil without calling ctx.JSON or ctx.Error.
		slot.Respond(500, errorBody("handler_no_response", fmt.Errorf("handler returned without sending a response")))
		return
	}

	slot.Respond(uint16(rw.status), rw.body)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

func notFoundBody(path string) []byte {
	b, _ := json.Marshal(map[string]string{
		"error":   "not_found",
		"message": fmt.Sprintf("no handler for path: %s", path),
	})
	return b
}

func errorBody(code string, cause error) []byte {
	msg := ""
	if cause != nil {
		msg = cause.Error()
	}
	b, _ := json.Marshal(map[string]string{
		"error":   code,
		"message": msg,
	})
	return b
}

// decodeAuth parses the raw auth_claims bytes. In Phase 1 these will be
// verified Ed25519 JWT payloads; in Phase 0 they are treated as raw JSON.
func decodeAuth(raw []byte) AuthClaims {
	if len(raw) == 0 {
		return AuthClaims{}
	}
	var claims map[string]interface{}
	if err := json.Unmarshal(raw, &claims); err != nil {
		return AuthClaims{}
	}
	subject, _ := claims["sub"].(string)
	var roles []string
	if r, ok := claims["roles"].([]interface{}); ok {
		for _, v := range r {
			if s, ok := v.(string); ok {
				roles = append(roles, s)
			}
		}
	}
	return AuthClaims{Subject: subject, Roles: roles, Raw: claims}
}
