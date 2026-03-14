# workers-rs-mcp

An example [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server written in Rust, running on [Cloudflare Workers](https://workers.cloudflare.com/).

The server compiles to WebAssembly and runs on Cloudflare's edge infrastructure, handling MCP JSON-RPC requests over HTTP.

## How it works

HTTP routing is handled by [Axum](https://github.com/tokio-rs/axum) via the [workers-rs](https://github.com/cloudflare/workers-rs) SDK. MCP protocol logic is handled by the [`mcpserver`](https://crates.io/crates/mcpserver) crate.

### Endpoints

| Method | Path      | Description                        |
|--------|-----------|------------------------------------|
| GET    | `/`       | Server name and version            |
| GET    | `/healthz`| Health check — returns `OK`        |
| POST   | `/mcp`    | MCP JSON-RPC endpoint              |

### Session management

- `initialize` requests generate a new `mcp-session-id` (UUID v4) returned in the response header
- All subsequent requests should include the `mcp-session-id` header

### Tools

Includes one example tool:

- **echo** — echoes the input `message` back to the caller

## Getting started

### Prerequisites

- Rust with the `wasm32-unknown-unknown` target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/install-and-update/):
  ```sh
  npm install -g wrangler
  ```

### Local development

```sh
wrangler dev
```

### Deploy to Cloudflare Workers

```sh
wrangler deploy
```

Pushing to `main` also deploys automatically via GitHub Actions (requires `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets).

## Adding tools

1. Implement the `ToolHandler` trait in `src/lib.rs`:

```rust
struct MyToolHandler;

#[async_trait]
impl ToolHandler for MyToolHandler {
    async fn call(&self, args: Value, _context: Value) -> Result<ToolResult, McpError> {
        // your logic here
        Ok(text_result("result".to_string()))
    }
}
```

2. Register the tool with the MCP server:

```rust
mcp_router.handle_tool("my-tool", Arc::new(MyToolHandler));
```

3. Add the tool schema to `mcp/tools.json`.
