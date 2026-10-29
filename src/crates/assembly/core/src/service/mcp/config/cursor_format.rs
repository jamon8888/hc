use crate::service::mcp::server::MCPServerConfig;
use crate::util::errors::AlexResult;

pub(super) fn config_to_cursor_format(config: &MCPServerConfig) -> serde_json::Value {
    alex_services_integrations::mcp::config::config_to_cursor_format(config)
}

pub(super) fn parse_cursor_format(
    config: &serde_json::Value,
) -> AlexResult<Vec<MCPServerConfig>> {
    Ok(alex_services_integrations::mcp::config::parse_cursor_format(config))
}
