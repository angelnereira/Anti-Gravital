// Package brain provides the Go runtime layer of Anti-Gravital: the worker
// pool that dispatches ring buffer requests to application handler functions.
package brain

import "fmt"

// HandlerFunc is the function signature for Anti-Gravital request handlers.
// The handler must call exactly one response method on ctx before returning.
type HandlerFunc func(ctx Context) error

// HandlerError is a structured error type that carries an HTTP status code and
// a machine-readable error code string alongside the underlying error.
type HandlerError struct {
	Status int
	Code   string
	Cause  error
}

func (e *HandlerError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("[%d %s] %v", e.Status, e.Code, e.Cause)
	}
	return fmt.Sprintf("[%d %s]", e.Status, e.Code)
}

func (e *HandlerError) Unwrap() error {
	return e.Cause
}

// NewHandlerError creates a HandlerError with the given status, code, and cause.
func NewHandlerError(status int, code string, cause error) *HandlerError {
	return &HandlerError{Status: status, Code: code, Cause: cause}
}

// InternalError wraps an arbitrary error as a 500 Internal Server Error.
func InternalError(cause error) *HandlerError {
	return &HandlerError{Status: 500, Code: "internal_server_error", Cause: cause}
}

// NotFoundError returns a 404 Not Found error with the given resource description.
func NotFoundError(resource string) *HandlerError {
	return &HandlerError{
		Status: 404,
		Code:   "not_found",
		Cause:  fmt.Errorf("resource not found: %s", resource),
	}
}
