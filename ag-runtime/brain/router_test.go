package brain

import "testing"

func TestRouterExactMatch(t *testing.T) {
	r := NewRouter()
	called := false
	r.GET("/health", func(ctx Context) error {
		called = true
		return ctx.JSON(200, "ok")
	})
	h, ok := r.Match("GET", "/health")
	if !ok {
		t.Fatal("expected match for GET /health")
	}
	ctx, rw := newContext("GET", "/health", nil, AuthClaims{})
	_ = h(ctx)
	if !called {
		t.Error("handler was not called")
	}
	if rw.status != 200 {
		t.Errorf("status = %d, want 200", rw.status)
	}
}

func TestRouterNamedSegment(t *testing.T) {
	r := NewRouter()
	r.GET("/users/:id", func(ctx Context) error { return ctx.JSON(200, "ok") })

	if _, ok := r.Match("GET", "/users/42"); !ok {
		t.Error("expected match for /users/42")
	}
	if _, ok := r.Match("GET", "/users"); ok {
		t.Error("should not match /users (missing segment)")
	}
}

func TestRouterWildcard(t *testing.T) {
	r := NewRouter()
	r.GET("/static/*", func(ctx Context) error { return ctx.JSON(200, "ok") })

	if _, ok := r.Match("GET", "/static/a/b/c"); !ok {
		t.Error("expected wildcard match for /static/a/b/c")
	}
	if _, ok := r.Match("GET", "/other"); ok {
		t.Error("should not match /other")
	}
}

func TestRouterMethodMismatch(t *testing.T) {
	r := NewRouter()
	r.GET("/ping", func(ctx Context) error { return ctx.JSON(200, "ok") })

	if _, ok := r.Match("POST", "/ping"); ok {
		t.Error("should not match POST for a GET-only route")
	}
}

func TestRouterNoMatch(t *testing.T) {
	r := NewRouter()
	if _, ok := r.Match("GET", "/nonexistent"); ok {
		t.Error("empty router should not match anything")
	}
}
