//! Resolution of ACP agent commands that ship with the Alex product.
//!
//! Some ACP agents are packaged with the product, so a user does not have to
//! install them before they work. The packaged copy is Alex's default,
//! not an override of a deliberate choice: a client whose command the user
//! changed keeps the command the user chose.

use std::path::PathBuf;

use alex_core::service::bundled_tools::bundled_tool_package;

use super::builtin_clients::builtin_acp_client_preset;
use super::config::AcpClientConfig;
use super::requirements::resolve_configured_command;

/// The packaged command for a client, when the product ships one.
///
/// `config` is the client's saved configuration, or `None` for a builtin the
/// user has not configured yet — which is the same thing, because a builtin
/// the user has not configured still uses the preset's own command.
pub(crate) fn bundled_client_command(
    client_id: &str,
    config: Option<&AcpClientConfig>,
) -> Option<PathBuf> {
    let preset = builtin_acp_client_preset(client_id)?;
    let effective_command = config
        .map(|config| config.command.trim())
        .unwrap_or(preset.command);
    if effective_command != preset.command {
        return None;
    }
    let tool_id = preset.bundled_tool?;
    bundled_tool_package(tool_id).map(|package| package.entrypoint)
}

/// The command a client is launched with.
pub(crate) fn resolve_client_command(
    client_id: &str,
    config: &AcpClientConfig,
) -> PathBuf {
    bundled_client_command(client_id, Some(config))
        .unwrap_or_else(|| resolve_configured_command(&config.command, &config.env))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interpreter_config(command: &str) -> AcpClientConfig {
        AcpClientConfig {
            name: Some("Open Interpreter".to_string()),
            command: command.to_string(),
            args: vec!["acp".to_string()],
            env: Default::default(),
            enabled: true,
            readonly: false,
            subagent: Default::default(),
            permission_mode: Default::default(),
        }
    }

    #[test]
    fn a_user_chosen_command_is_never_replaced_by_the_packaged_one() {
        // The packaged copy is only the default. A command the user typed is
        // their choice, even when it happens to be the same name.
        let config = interpreter_config("/opt/homebrew/bin/interpreter");

        assert!(bundled_client_command("interpreter", Some(&config)).is_none());
    }

    #[test]
    fn a_client_without_a_packaged_copy_resolves_nothing() {
        let config = interpreter_config("opencode");

        assert!(bundled_client_command("opencode", Some(&config)).is_none());
    }

    #[test]
    fn an_unknown_client_resolves_nothing() {
        assert!(bundled_client_command("not-a-preset", None).is_none());
    }

    #[test]
    fn the_preset_command_falls_back_to_the_search_path() {
        // Without a packaged copy on this machine the preset still resolves
        // the way it always did: through the user's PATH.
        let config = interpreter_config("interpreter");
        let resolved = resolve_client_command("interpreter", &config);

        assert!(
            resolved.ends_with("interpreter"),
            "unexpected resolved command: {resolved:?}"
        );
    }
}
