//! Provider-authenticated standalone web search (`web.run` compatibility).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::types::output::WebRunOutput;
use crate::types::requirements::{Expr, ToolRequirement};
use crate::types::tool::{ToolKind, ToolNamespace};
use crate::types::tool_metadata::shared_resources;

pub const WEB_RUN_TOOL_NAME: &str = "web__run";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, schemars::JsonSchema)]
pub struct SearchCommands {
    /// Query the internet search engine. At most four queries per call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<Vec<SearchQuery>>,
    /// Query the image search engine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_query: Option<Vec<SearchQuery>>,
    /// Open pages by reference id or URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<Vec<OpenOperation>>,
    /// Open numbered links from previously opened pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click: Option<Vec<ClickOperation>>,
    /// Find text patterns in pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub find: Option<Vec<FindOperation>>,
    /// Take screenshots of PDF pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<Vec<ScreenshotOperation>>,
    /// Look up prices for stock symbols and other assets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finance: Option<Vec<FinanceOperation>>,
    /// Look up weather forecasts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather: Option<Vec<WeatherOperation>>,
    /// Look up sports schedules and standings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sports: Option<Vec<SportsOperation>>,
    /// Get time for UTC offsets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<Vec<TimeOperation>>,
    /// Control response detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_length: Option<SearchResponseLength>,
}

impl SearchCommands {
    pub fn is_empty(&self) -> bool {
        self.search_query.as_ref().is_none_or(Vec::is_empty)
            && self.image_query.as_ref().is_none_or(Vec::is_empty)
            && self.open.as_ref().is_none_or(Vec::is_empty)
            && self.click.as_ref().is_none_or(Vec::is_empty)
            && self.find.as_ref().is_none_or(Vec::is_empty)
            && self.screenshot.as_ref().is_none_or(Vec::is_empty)
            && self.finance.as_ref().is_none_or(Vec::is_empty)
            && self.weather.as_ref().is_none_or(Vec::is_empty)
            && self.sports.as_ref().is_none_or(Vec::is_empty)
            && self.time.as_ref().is_none_or(Vec::is_empty)
    }

    pub fn summary(&self) -> String {
        if let Some(query) = self
            .search_query
            .as_ref()
            .and_then(|queries| queries.first())
            .or_else(|| {
                self.image_query
                    .as_ref()
                    .and_then(|queries| queries.first())
            })
        {
            return query.q.clone();
        }
        if let Some(open) = self.open.as_ref().and_then(|items| items.first()) {
            return format!("open {}", open.ref_id);
        }
        if let Some(finance) = self.finance.as_ref().and_then(|items| items.first()) {
            return format!("finance {}", finance.ticker);
        }
        if let Some(weather) = self.weather.as_ref().and_then(|items| items.first()) {
            return format!("weather {}", weather.location);
        }
        "standalone web search".to_string()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchQuery {
    pub q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recency: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct OpenOperation {
    pub ref_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineno: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ClickOperation {
    pub ref_id: String,
    pub id: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct FindOperation {
    pub ref_id: String,
    pub pattern: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreenshotOperation {
    pub ref_id: String,
    pub pageno: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct FinanceOperation {
    pub ticker: String,
    pub r#type: FinanceAssetType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FinanceAssetType {
    Equity,
    Fund,
    Crypto,
    Index,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct WeatherOperation {
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SportsOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<SportsToolName>,
    pub r#fn: SportsFunction,
    pub league: SportsLeague,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_games: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SportsToolName {
    Sports,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SportsFunction {
    Schedule,
    Standings,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SportsLeague {
    Nba,
    Wnba,
    Nfl,
    Nhl,
    Mlb,
    Epl,
    Ncaamb,
    Ncaawb,
    Ipl,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TimeOperation {
    pub utc_offset: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchResponseLength {
    Short,
    Medium,
    Long,
}

pub type StandaloneWebSearchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

pub trait StandaloneWebSearchBackend: Send + Sync + std::fmt::Debug {
    fn search<'a>(&'a self, commands: SearchCommands) -> StandaloneWebSearchFuture<'a>;
}

#[derive(Clone)]
pub struct StandaloneWebSearchBackendResource(pub Arc<dyn StandaloneWebSearchBackend>);

impl std::fmt::Debug for StandaloneWebSearchBackendResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("StandaloneWebSearchBackendResource")
            .field(&"<backend>")
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct WebRunTool;

impl crate::types::tool_metadata::ToolMetadata for WebRunTool {
    fn kind(&self) -> ToolKind {
        ToolKind::WebSearch
    }

    fn tool_namespace(&self) -> ToolNamespace {
        ToolNamespace::GrokBuild
    }

    fn description_template(&self) -> &str {
        "Access the internet with batched search_query, image_query, open, click, find, screenshot, finance, weather, sports, and time commands. Reuse returned reference IDs only in later web__run calls. Prefer batching related commands in one call."
    }

    fn requires_expr(&self) -> Expr<ToolRequirement> {
        Expr::True
    }
}

impl xai_tool_runtime::Tool for WebRunTool {
    type Args = SearchCommands;
    type Output = WebRunOutput;

    fn id(&self) -> xai_tool_protocol::ToolId {
        xai_tool_protocol::ToolId::new(WEB_RUN_TOOL_NAME).expect("valid tool id")
    }

    fn description(
        &self,
        _ctx: &xai_tool_runtime::ListToolsContext,
    ) -> xai_tool_types::ToolDescription {
        xai_tool_types::ToolDescription::new(
            WEB_RUN_TOOL_NAME,
            crate::types::tool_metadata::ToolMetadata::sanitized_description_template(self),
        )
    }

    fn capabilities(&self) -> xai_tool_protocol::ToolCapabilities {
        xai_tool_protocol::ToolCapabilities {
            is_read_only: true,
            tool_scope: Some(xai_tool_protocol::ToolScope::Read),
            ..Default::default()
        }
    }

    #[tracing::instrument(name = "tool.web_run", skip_all)]
    async fn run(
        &self,
        ctx: xai_tool_runtime::ToolCallContext,
        commands: SearchCommands,
    ) -> Result<WebRunOutput, xai_tool_runtime::ToolError> {
        if commands
            .search_query
            .as_ref()
            .is_some_and(|queries| queries.len() > 4)
        {
            return Err(xai_tool_runtime::ToolError::execution(
                self.id(),
                "web__run accepts at most four search_query entries per call",
            ));
        }

        let backend = {
            let resources = shared_resources(&ctx)?;
            let resources = resources.lock().await;
            resources
                .require::<StandaloneWebSearchBackendResource>()?
                .0
                .clone()
        };
        let output = backend.search(commands).await.map_err(|error| {
            xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new(WEB_RUN_TOOL_NAME).expect("valid tool id"),
                error,
            )
        })?;
        Ok(WebRunOutput { output })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::SearchCommands;
    use super::StandaloneWebSearchBackend;
    use super::StandaloneWebSearchBackendResource;
    use super::StandaloneWebSearchFuture;
    use super::WebRunTool;
    use crate::types::resources::Resources;
    use crate::types::tool_metadata::test_ctx_with_call_id;

    #[derive(Debug)]
    struct FakeBackend;

    impl StandaloneWebSearchBackend for FakeBackend {
        fn search<'a>(&'a self, commands: SearchCommands) -> StandaloneWebSearchFuture<'a> {
            Box::pin(async move { Ok(format!("result for {}", commands.summary())) })
        }
    }

    #[test]
    fn search_commands_match_upstream_wire_shape() {
        let commands: SearchCommands = serde_json::from_value(json!({
            "search_query": [
                {"q": "Open Grok", "recency": 7, "domains": ["github.com"]},
                {"q": "Codex code mode"}
            ],
            "open": [{"ref_id": "turn0search0", "lineno": 12}],
            "response_length": "medium"
        }))
        .unwrap();
        assert_eq!(commands.summary(), "Open Grok");
        assert_eq!(
            serde_json::to_value(commands).unwrap()["response_length"],
            "medium"
        );
    }

    #[tokio::test]
    async fn tool_returns_compact_backend_output() {
        let mut resources = Resources::new();
        resources.insert(StandaloneWebSearchBackendResource(Arc::new(FakeBackend)));
        let output = xai_tool_runtime::Tool::run(
            &WebRunTool,
            test_ctx_with_call_id(resources.into_shared(), "web-run-call"),
            SearchCommands {
                search_query: Some(vec![super::SearchQuery {
                    q: "Open Grok".into(),
                    recency: None,
                    domains: None,
                }]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(output.output, "result for Open Grok");
    }
}
