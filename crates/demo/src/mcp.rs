//! MCP tools at /mcp. Each tool is a thin adapter over the same domain functions the web pages use.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use oc_kit::auth::CurrentUser;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, RoleServer, schemars, tool, tool_router};
use serde::Deserialize;

use crate::domain::events::{self, Cursor};
use crate::domain::notes::{self, NoteInput};
use crate::domain::{cases, now, query};
use crate::state::AppState;

#[derive(Clone)]
pub struct Mcp {
    st: AppState,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SearchArgs {
    /// Case to search, from list_cases.
    pub case_id: i64,
    /// Search such as `host:ws-01 -user:SYSTEM powershell`; empty matches everything.
    #[serde(default)]
    pub query: String,
    /// 1 to 200, default 50.
    pub limit: Option<i64>,
    /// next_cursor from the previous call.
    pub cursor: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct EventArgs {
    pub id: i64,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct NoteArgs {
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub case_id: Option<i64>,
}

fn json(v: impl serde::Serialize) -> Result<CallToolResult, ErrorData> {
    let text =
        serde_json::to_string(&v).map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn fail(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::invalid_params(e.to_string(), None)
}

#[tool_router(server_handler)]
impl Mcp {
    #[tool(description = "List investigation cases with their event and note counts.")]
    async fn list_cases(&self) -> Result<CallToolResult, ErrorData> {
        json(cases::list(&self.st.pool).await.map_err(fail)?)
    }

    #[tool(
        description = "Search a case's events, newest first. Returns items and next_cursor for the next page."
    )]
    async fn search_events(
        &self,
        Parameters(a): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let q = query::parse(&a.query).map_err(fail)?;
        let cursor = a
            .cursor
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(fail)?;
        let page = events::search(
            &self.st.pool,
            a.case_id,
            &q,
            cursor,
            a.limit.unwrap_or(50).clamp(1, 200),
        )
        .await
        .map_err(fail)?;
        json(page)
    }

    #[tool(description = "Get every field of one event by id.")]
    async fn get_event(
        &self,
        Parameters(a): Parameters<EventArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        json(events::get(&self.st.pool, a.id).await.map_err(fail)?)
    }

    #[tool(
        description = "Add a note, optionally attached to a case. The note is authored by the token's user."
    )]
    async fn add_note(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(a): Parameters<NoteArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let user = ctx
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|p| p.extensions.get::<CurrentUser>())
            .map(|u| u.0.id)
            .ok_or_else(|| ErrorData::invalid_request("Sign in first.", None))?;
        let input = NoteInput {
            case_id: a.case_id,
            title: a.title,
            body: a.body,
            tags: Vec::new(),
        };
        let id = notes::create(&self.st.pool, user, &input, now())
            .await
            .map_err(fail)?;
        json(notes::get(&self.st.pool, id).await.map_err(fail)?)
    }
}

/// MCP clients authenticate with a bearer token; the 401 points OAuth-capable clients at the IdP when one is configured.
async fn require_user(State(st): State<AppState>, req: Request, next: Next) -> Response {
    let (mut parts, body) = req.into_parts();
    match <CurrentUser as axum::extract::FromRequestParts<AppState>>::from_request_parts(
        &mut parts, &st,
    )
    .await
    {
        Ok(u) => {
            parts.extensions.insert(u);
            next.run(Request::from_parts(parts, body)).await
        }
        Err(rejection) => rejection.into_response(),
    }
}

pub fn router(st: AppState) -> axum::Router<AppState> {
    let mut hosts = vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ];
    hosts.push(st.auth.dep.rp_id.clone());
    let config = StreamableHttpServerConfig::default().with_allowed_hosts(hosts);
    let service = {
        let st = st.clone();
        StreamableHttpService::new(
            move || Ok(Mcp { st: st.clone() }),
            Arc::new(LocalSessionManager::default()),
            config,
        )
    };
    axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(st, require_user))
}
