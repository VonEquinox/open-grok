//! Data APIs for `open-grok models`. Clients own display.

use agent_client_protocol as acp;
use anyhow::Result;
use xai_acp_lib::{AcpAgentTx, acp_send};

use crate::agent::config::Config as AgentConfig;

/// Status for the `og models` banner.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthStatus {
    /// Catalog key of a model with an explicit credential in config.toml.
    ModelCredentials(String),
    MissingModelCredentials,
}

impl AuthStatus {
    pub fn resolve(agent_config: &AgentConfig) -> Self {
        let models = crate::agent::config::resolve_model_list(agent_config, None);
        let configured = agent_config
            .models
            .default
            .as_ref()
            .and_then(|default| models.get(default).map(|entry| (default, entry)))
            .filter(|(_, entry)| entry.has_own_credentials())
            .map(|(name, _)| name.clone())
            .or_else(|| {
                models
                    .iter()
                    .find_map(|(name, entry)| entry.has_own_credentials().then(|| name.clone()))
            });
        configured
            .map(Self::ModelCredentials)
            .unwrap_or(Self::MissingModelCredentials)
    }
}

/// Fetch model state (available models + default) over an ACP channel.
pub async fn list_models(
    acp_tx: &AcpAgentTx,
    client_type: &str,
    client_version: &str,
) -> Result<acp::SessionModelState> {
    let init_resp: acp::InitializeResponse = acp_send(
        acp::InitializeRequest::new(acp::ProtocolVersion::V1)
            .client_capabilities(
                acp::ClientCapabilities::new()
                    .fs(acp::FileSystemCapabilities::new())
                    .terminal(false),
            )
            .meta(
                serde_json::json!({
                    "clientType": client_type,
                    "clientVersion": client_version,
                })
                .as_object()
                .cloned(),
            ),
        acp_tx,
    )
    .await?;

    let model_state = init_resp
        .meta
        .and_then(|m| m.get("modelState").cloned())
        .ok_or_else(|| anyhow::anyhow!("InitializeResponse missing modelState"))?;
    let state: acp::SessionModelState = serde_json::from_value(model_state)
        .map_err(|e| anyhow::anyhow!("Failed to parse modelState: {}", e))?;

    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::config::Config;

    fn config_from_toml(toml_src: &str) -> Config {
        let toml: toml::Value = toml::from_str(toml_src).unwrap();
        Config::new_from_toml_cfg(&toml).expect("config should parse")
    }

    #[test]
    fn configured_default_model_api_key_is_reported() {
        let cfg = config_from_toml(
            r#"
            [models]
            default = "custom"

            [model.custom]
            model = "remote-model"
            provider = "codex"
            api_backend = "responses"
            api_key = "test-key"
            "#,
        );
        assert_eq!(
            AuthStatus::resolve(&cfg),
            AuthStatus::ModelCredentials("custom".to_owned())
        );
    }

    #[test]
    fn missing_model_key_is_reported_without_account_auth() {
        let cfg = config_from_toml(
            r#"
            [models]
            default = "custom"

            [model.custom]
            model = "remote-model"
            provider = "codex"
            api_backend = "responses"
            "#,
        );
        assert_eq!(
            AuthStatus::resolve(&cfg),
            AuthStatus::MissingModelCredentials
        );
    }
}
