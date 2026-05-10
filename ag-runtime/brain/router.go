package brain

import "strings"

// route holds a registered handler along with its method and path pattern.
type route struct {
	method  string
	pattern string
	handler HandlerFunc
}

// Router maps (method, path) pairs to HandlerFuncs. In Phase 0 this is a
// simple linear-scan prefix router. It will be replaced by a radix trie in
// Phase 2 when routing performance becomes measurable.
type Router struct {
	routes []route
}

// NewRouter returns an initialised Router.
func NewRouter() *Router {
	return &Router{}
}

// Handle registers handler for the given HTTP method and path pattern.
//
// Path patterns support a single trailing wildcard ("*") and named segments
// prefixed with a colon (":id"). In Phase 0, only exact and prefix matching
// is implemented.
func (r *Router) Handle(method, pattern string, handler HandlerFunc) {
	r.routes = append(r.routes, route{
		method:  strings.ToUpper(method),
		pattern: pattern,
		handler: handler,
	})
}

// GET registers a GET handler.
func (r *Router) GET(pattern string, h HandlerFunc) { r.Handle("GET", pattern, h) }

// POST registers a POST handler.
func (r *Router) POST(pattern string, h HandlerFunc) { r.Handle("POST", pattern, h) }

// PUT registers a PUT handler.
func (r *Router) PUT(pattern string, h HandlerFunc) { r.Handle("PUT", pattern, h) }

// PATCH registers a PATCH handler.
func (r *Router) PATCH(pattern string, h HandlerFunc) { r.Handle("PATCH", pattern, h) }

// DELETE registers a DELETE handler.
func (r *Router) DELETE(pattern string, h HandlerFunc) { r.Handle("DELETE", pattern, h) }

// Match returns the handler for the given method and path, or (nil, false) if
// none was found.
func (r *Router) Match(method, path string) (HandlerFunc, bool) {
	method = strings.ToUpper(method)
	for _, rt := range r.routes {
		if rt.method != method && rt.method != "*" {
			continue
		}
		if matchPath(rt.pattern, path) {
			return rt.handler, true
		}
	}
	return nil, false
}

// matchPath returns true if path matches the given pattern.
//
// Rules:
//   - Exact match: "/users" matches "/users"
//   - Named segment: "/users/:id" matches "/users/42" (any single segment)
//   - Wildcard: "/static/*" matches "/static/a/b/c"
func matchPath(pattern, path string) bool {
	if pattern == path {
		return true
	}

	pp := splitPath(pattern)
	rp := splitPath(path)

	if len(pp) == 0 {
		return len(rp) == 0
	}

	// Trailing wildcard: pattern must be a prefix.
	if pp[len(pp)-1] == "*" {
		if len(rp) < len(pp)-1 {
			return false
		}
		for i, seg := range pp[:len(pp)-1] {
			if !segmentMatches(seg, rp[i]) {
				return false
			}
		}
		return true
	}

	if len(pp) != len(rp) {
		return false
	}
	for i, seg := range pp {
		if !segmentMatches(seg, rp[i]) {
			return false
		}
	}
	return true
}

func segmentMatches(pattern, value string) bool {
	return pattern == "*" || strings.HasPrefix(pattern, ":") || pattern == value
}

func splitPath(path string) []string {
	path = strings.TrimPrefix(path, "/")
	if path == "" {
		return nil
	}
	return strings.Split(path, "/")
}
