# ag-wasm/host

The WASM plugin host loads and executes WebAssembly modules within the Rust
Shield layer. Plugins can intercept requests and responses at the network edge
without being able to corrupt the main process.

Implementation is scheduled for Phase 4.

## Plugin Capabilities (Planned)

Plugins run in a sandboxed WASM environment. The host grants explicit
capabilities:

- **Request inspection**: Read request headers, path, and body.
- **Response modification**: Modify response headers and body.
- **Early return**: Short-circuit the request with a custom response.
- **Logging**: Write to the structured log stream.

Plugins cannot:

- Access the filesystem beyond a configured sandbox directory.
- Open network connections (all network I/O goes through the host).
- Access shared memory directly (they communicate via a well-defined ABI).
- Affect other requests or the global state.

## Plugin Interface (Planned)

```rust
pub trait AgPlugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn on_request(&mut self, req: &mut PluginRequest) -> PluginResult;
    fn on_response(&mut self, res: &mut PluginResponse) -> PluginResult;
}
```

This interface is the same regardless of the language the plugin is written in.
Plugins can be written in Rust, Go, C, AssemblyScript, or any language that
compiles to WASM.
