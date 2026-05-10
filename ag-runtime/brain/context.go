package brain

import (
	"encoding/json"
	"fmt"
)

// AuthClaims holds the decoded JWT or session claims passed through the
// ring buffer from the Rust Shield layer.
type AuthClaims struct {
	// Subject is the primary identity identifier (e.g. user ID or passkey ID).
	Subject string
	// Roles holds the RBAC roles assigned to this subject.
	Roles []string
	// Raw holds all claims decoded from the JWT payload for extension fields.
	Raw map[string]interface{}
}

// Context is the interface provided to every Anti-Gravital handler. It gives
// type-safe access to the request data validated by the Shield layer and
// provides response helpers.
type Context interface {
	// Method returns the HTTP method as a string ("GET", "POST", etc.).
	Method() string
	// Path returns the request path ("/users/42").
	Path() string
	// Body returns the raw request body bytes.
	Body() []byte
	// Auth returns the decoded authentication claims. Returns zero-value
	// AuthClaims for unauthenticated requests.
	Auth() AuthClaims

	// JSON serialises v as JSON and writes it as the response with the given
	// HTTP status code. Must be called exactly once per request.
	JSON(status int, v interface{}) error
	// Error sends an error response. Must be called exactly once per request.
	Error(status int, code string, cause error) error
}

// responseWriter is the internal type that accumulates a response and flushes
// it back to the Dispatcher once the handler returns.
type responseWriter struct {
	written bool
	status  int
	body    []byte
	err     error
}

func (r *responseWriter) writeJSON(status int, v interface{}) error {
	if r.written {
		return fmt.Errorf("context: JSON called more than once")
	}
	b, err := json.Marshal(v)
	if err != nil {
		return fmt.Errorf("context: JSON marshal: %w", err)
	}
	r.status = status
	r.body = b
	r.written = true
	return nil
}

func (r *responseWriter) writeError(status int, code string, cause error) error {
	if r.written {
		return fmt.Errorf("context: Error called more than once")
	}
	type errBody struct {
		Error   string `json:"error"`
		Message string `json:"message,omitempty"`
	}
	msg := ""
	if cause != nil {
		msg = cause.Error()
	}
	b, _ := json.Marshal(errBody{Error: code, Message: msg})
	r.status = status
	r.body = b
	r.written = true
	r.err = cause
	return nil
}

// agContext implements Context. One instance is created per request.
type agContext struct {
	method string
	path   string
	body   []byte
	auth   AuthClaims
	resp   *responseWriter
}

func newContext(method, path string, body []byte, auth AuthClaims) (*agContext, *responseWriter) {
	rw := &responseWriter{}
	return &agContext{
		method: method,
		path:   path,
		body:   body,
		auth:   auth,
		resp:   rw,
	}, rw
}

func (c *agContext) Method() string       { return c.method }
func (c *agContext) Path() string         { return c.path }
func (c *agContext) Body() []byte         { return c.body }
func (c *agContext) Auth() AuthClaims     { return c.auth }

func (c *agContext) JSON(status int, v interface{}) error {
	return c.resp.writeJSON(status, v)
}

func (c *agContext) Error(status int, code string, cause error) error {
	return c.resp.writeError(status, code, cause)
}

// httpMethodString converts the numeric method code from the ring buffer slot
// to an HTTP method string.
func httpMethodString(code uint8) string {
	switch code {
	case 0:
		return "GET"
	case 1:
		return "POST"
	case 2:
		return "PUT"
	case 3:
		return "PATCH"
	case 4:
		return "DELETE"
	case 5:
		return "HEAD"
	case 6:
		return "OPTIONS"
	default:
		return "GET"
	}
}
