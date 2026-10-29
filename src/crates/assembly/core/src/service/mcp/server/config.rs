//! MCP server configuration types.

use crate::util::errors::AlexError;

use alex_services_integrations::mcp::server::MCPServerConfigValidationError;
pub use alex_services_integrations::mcp::server::{
    MCPImportOrigin, MCPServerConfig, MCPServerOAuthConfig, MCPServerTimeouts, MCPServerTransport,
    MCPServerXaaConfig,
};

impl From<MCPServerConfigValidationError> for AlexError {
    fn from(error: MCPServerConfigValidationError) -> Self {
        Self::Configuration(error.to_string())
    }
}
