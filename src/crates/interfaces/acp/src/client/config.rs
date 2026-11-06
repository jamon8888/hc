use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use alex_runtime_ports::ModelPrivacyRoute;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientConfigFile {
    #[serde(default)]
    pub acp_clients: HashMap<String, AcpClientConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientSubagentConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_for: Option<String>,
}

impl Default for AcpClientSubagentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            description: None,
            best_for: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientConfig {
    #[serde(default)]
    pub name: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub subagent: AcpClientSubagentConfig,
    #[serde(default)]
    pub permission_mode: AcpClientPermissionMode,
    /// Explicit route of the external ACP client's model.
    ///
    /// Missing legacy configuration is intentionally unknown: the host must
    /// refuse the prompt instead of guessing that an agent is local.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_privacy_route")]
    pub privacy_route: Option<ModelPrivacyRoute>,
}

fn deserialize_privacy_route<'de, D>(
    deserializer: D,
) -> Result<Option<ModelPrivacyRoute>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value.and_then(|value| value.as_str().map(str::to_owned)) {
        None => None,
        Some(route) if route == "api" => Some(ModelPrivacyRoute::Api),
        Some(route) if route == "local" => Some(ModelPrivacyRoute::Local),
        Some(_) => Some(ModelPrivacyRoute::Unknown),
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcpClientPermissionMode {
    #[default]
    Ask,
    AllowOnce,
    RejectOnce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientInfo {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
    pub readonly: bool,
    pub subagent: AcpClientSubagentConfig,
    pub permission_mode: AcpClientPermissionMode,
    #[serde(default)]
    pub privacy_route: Option<ModelPrivacyRoute>,
    pub status: AcpClientStatus,
    pub tool_name: String,
    pub session_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientRequirementProbe {
    pub id: String,
    pub tool: AcpRequirementProbeItem,
    #[serde(default)]
    pub adapter: Option<AcpRequirementProbeItem>,
    pub runnable: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAcpClientRequirementSnapshot {
    pub connection_id: String,
    pub last_probed_at: u64,
    #[serde(default)]
    pub probes: Vec<AcpClientRequirementProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpRequirementProbeItem {
    pub name: String,
    pub installed: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcpClientStatus {
    Configured,
    Starting,
    Running,
    Stopped,
    Failed,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{AcpClientConfigFile, AcpClientPermissionMode};

    #[test]
    fn permission_mode_default_remains_ask_on_the_wire() {
        let mode = AcpClientPermissionMode::default();

        assert_eq!(mode, AcpClientPermissionMode::Ask);
        assert_eq!(serde_json::to_string(&mode).unwrap(), "\"ask\"");
    }

    #[test]
    fn legacy_client_config_keeps_external_agent_tool_enabled() {
        let config: AcpClientConfigFile = serde_json::from_value(serde_json::json!({
            "acpClients": {
                "codex": {
                    "command": "codex",
                    "enabled": true
                }
            }
        }))
        .unwrap();

        let codex = config.acp_clients.get("codex").unwrap();
        assert!(codex.subagent.enabled);
        assert!(codex.subagent.description.is_none());
        assert!(codex.subagent.best_for.is_none());
    }

    #[test]
    fn subagent_profile_round_trips_with_camel_case_fields() {
        let config: AcpClientConfigFile = serde_json::from_value(serde_json::json!({
            "acpClients": {
                "codex": {
                    "command": "codex",
                    "subagent": {
                        "enabled": true,
                        "description": "Implements complex code changes",
                        "bestFor": "Cross-file refactors and difficult debugging"
                    }
                }
            }
        }))
        .unwrap();

        let serialized = serde_json::to_value(config).unwrap();
        assert_eq!(
            serialized["acpClients"]["codex"]["subagent"]["bestFor"],
            "Cross-file refactors and difficult debugging"
        );
    }
}
