use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use mcpserver::{
    text_result, JsonRpcRequest, McpError, McpResponse, Server, ToolHandler, ToolResult,
};
use serde_json::Value;
use tower_service::Service;
use uuid::Uuid;
use worker::event;

struct EchoHandler;

#[async_trait]
impl ToolHandler for EchoHandler {
    async fn call(&self, args: Value, _context: Value) -> Result<ToolResult, McpError> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("(empty)");
        Ok(text_result(format!("echo: {}", message)))
    }
}

#[event(fetch)]
async fn fetch(
    req: worker::HttpRequest,
    _env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<Response<Body>> {
    let mut mcp_router = Server::builder()
        .server_info(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .tools_json(include_bytes!("../mcp/tools.json"))
        .build();

    mcp_router.handle_tool("echo", Arc::new(EchoHandler));

    let mcp_server = Arc::new(mcp_router);

    let mut router = Router::new()
        .route("/healthz", get(|| async { "OK" }))
        .route("/mcp", post(handle_mcp))
        .route("/", get(root))
        .with_state(mcp_server);

    Ok(router.call(req).await?)
}

pub async fn root() -> &'static str {
    concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"))
}

async fn handle_mcp(
    State(mcp_server): State<Arc<Server>>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> Response<Body> {
    // Session management: create on initialize, pass through otherwise.
    let session_id = if req.method == "initialize" {
        let id = Uuid::new_v4().to_string();
        Some(id)
    } else {
        headers
            .get("mcp-session-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    };

    // Build request context from the HTTP layer.
    // In a real app, this would contain decoded JWT claims, tenant info, etc.
    let context = Default::default();

    // The library handles all MCP protocol logic.
    // McpResponse holds Arc references to pre-serialized JSON for cached
    // endpoints — zero data copying.
    let resp: McpResponse = mcp_server.handle(req, context).await;

    // Notifications get 202 with no body.
    if resp.is_notification() {
        return (StatusCode::ACCEPTED, Body::empty()).into_response();
    }

    // McpResponse implements Serialize — cached results are embedded verbatim.
    let mut response = Json(&resp).into_response();
    if let Some(sid) = session_id {
        response
            .headers_mut()
            .insert("mcp-session-id", sid.parse().unwrap());
    }
    response
}
