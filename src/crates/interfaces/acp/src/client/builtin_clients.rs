use std::collections::HashMap;

use super::config::{AcpClientConfig, AcpClientConfigFile, AcpClientPermissionMode};

const CLAUDE_ACP_PACKAGE: &str = "@agentclientprotocol/claude-agent-acp";
const CLAUDE_ACP_ARGS: &[&str] = &["--yes", "@agentclientprotocol/claude-agent-acp@latest"];
const LEGACY_CLAUDE_ACP_ARGS: &[&str] = &["--yes", "@zed-industries/claude-code-acp@latest"];
const CODEX_ACP_PACKAGE: &str = "@agentclientprotocol/codex-acp";
const CODEX_ACP_ARGS: &[&str] = &["--yes", "@agentclientprotocol/codex-acp@latest"];
const LEGACY_CODEX_ACP_ARGS: &[&str] = &["--yes", "@zed-industries/codex-acp@latest"];

pub(crate) struct BuiltinAcpClientPreset {
    pub(crate) id: &'static str,
    pub(crate) command: &'static str,
    pub(crate) args: &'static [&'static str],
    pub(crate) tool_command: &'static str,
    /// npm package Alex can install on the user's behalf. `None` means the
    /// agent is user-managed (Alex only provides the integration, the user
    /// installs the CLI themselves) — the UI then shows no one-click installer.
    pub(crate) install_package: Option<&'static str>,
    pub(crate) adapter_package: Option<&'static str>,
    pub(crate) adapter_bin: Option<&'static str>,
    /// A profile directory Alex ships and copies into the agent's own home
    /// before launching it. `None` — every preset but dsh — means the CLI is
    /// self-contained and the command runs as-is.
    pub(crate) bundled_profile: Option<&'static str>,
    /// A tool package that ships with the Alex product, resolved at
    /// runtime instead of from the user's PATH. `None` means the agent's CLI
    /// comes from the user's machine.
    pub(crate) bundled_tool: Option<&'static str>,
}

/// The profile directory Alex materializes for DeepSeek Harness. See
/// `dsh_profile.rs`: the bridge ships with Alex, the harness does not.
pub(crate) const DSH_BUNDLED_PROFILE: &str = "alex-acp";

/// The bundled tool package that carries the Open Interpreter agent. The id
/// matches the directory the build step packages it under, under
/// `resources/interpreter`.
pub(crate) const INTERPRETER_BUNDLED_TOOL: &str = "interpreter";

const BUILTIN_ACP_CLIENT_PRESETS: &[BuiltinAcpClientPreset] = &[
    BuiltinAcpClientPreset {
        id: "opencode",
        command: "opencode",
        args: &["acp"],
        tool_command: "opencode",
        install_package: Some("opencode-ai"),
        adapter_package: None,
        adapter_bin: None,
        bundled_profile: None,
        bundled_tool: None,
    },
    // DeepSeek Harness (dsh) — the harness has no ACP entry point of its own,
    // so Alex ships one as a dsh PROFILE (packages/dsh-acp) and launches it
    // through the user's own installation. The model and the API key stay in
    // dsh, where the user configured them; Alex stores neither. Installable
    // from npm like codex, hence install_package; native ACP, hence no adapter.
    BuiltinAcpClientPreset {
        id: "dsh",
        command: "dsh",
        args: &["--profile", DSH_BUNDLED_PROFILE],
        tool_command: "dsh",
        install_package: Some("@deepseek-ai/dsh"),
        adapter_package: None,
        adapter_bin: None,
        bundled_profile: Some(DSH_BUNDLED_PROFILE),
        bundled_tool: None,
    },
    // Oh My Pi (omp) — a terminal coding agent that speaks ACP natively via
    // `omp acp` (no adapter needed, like opencode). User-managed: omp targets
    // the bun runtime (installed via `bun install -g @oh-my-pi/pi-coding-agent`
    // or `curl -fsSL https://omp.sh/install | sh`), which Alex's npm-based
    // installer cannot provide — so install_package is None and Alex only
    // detects `omp` on PATH and launches it. https://github.com/can1357/oh-my-pi
    BuiltinAcpClientPreset {
        id: "omp",
        command: "omp",
        args: &["acp"],
        tool_command: "omp",
        install_package: None,
        adapter_package: None,
        adapter_bin: None,
        bundled_profile: None,
        bundled_tool: None,
    },
    // Open Interpreter — a Rust coding agent (an OpenAI Codex fork, Apache-2.0)
    // that speaks ACP natively via `interpreter acp` (no adapter needed, like
    // opencode and omp). Alex packages it with the product
    // (resources/interpreter), so a user does not have to install it first;
    // there is no npm package, so install_package is None and there is no
    // one-click installer to offer. Its draw is harness emulation — swapping the
    // model-facing prompt/tool surface per model family (kimi-code, qwen-code,
    // deepseek-tui, zcode, swe-agent) to get work out of low-cost models.
    // https://github.com/openinterpreter/openinterpreter
    BuiltinAcpClientPreset {
        id: "interpreter",
        command: "interpreter",
        args: &["acp"],
        tool_command: "interpreter",
        install_package: None,
        adapter_package: None,
        adapter_bin: None,
        bundled_profile: None,
        bundled_tool: Some(INTERPRETER_BUNDLED_TOOL),
    },
    BuiltinAcpClientPreset {
        id: "claude-code",
        command: "npx",
        args: CLAUDE_ACP_ARGS,
        tool_command: "claude",
        install_package: Some("@anthropic-ai/claude-code"),
        adapter_package: Some(CLAUDE_ACP_PACKAGE),
        adapter_bin: Some("claude-agent-acp"),
        bundled_profile: None,
        bundled_tool: None,
    },
    BuiltinAcpClientPreset {
        id: "codex",
        command: "npx",
        args: CODEX_ACP_ARGS,
        tool_command: "codex",
        install_package: Some("@openai/codex"),
        adapter_package: Some(CODEX_ACP_PACKAGE),
        adapter_bin: Some("codex-acp"),
        bundled_profile: None,
        bundled_tool: None,
    },
];

pub(crate) fn builtin_client_ids() -> impl Iterator<Item = &'static str> {
    BUILTIN_ACP_CLIENT_PRESETS.iter().map(|preset| preset.id)
}

pub(crate) fn builtin_acp_client_preset(
    client_id: &str,
) -> Option<&'static BuiltinAcpClientPreset> {
    BUILTIN_ACP_CLIENT_PRESETS
        .iter()
        .find(|preset| preset.id == client_id)
}

pub(crate) fn default_config_for_builtin_client(client_id: &str) -> Option<AcpClientConfig> {
    let preset = builtin_acp_client_preset(client_id)?;
    Some(AcpClientConfig {
        name: None,
        command: preset.command.to_string(),
        args: preset
            .args
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        env: HashMap::new(),
        enabled: true,
        readonly: false,
        subagent: Default::default(),
        permission_mode: AcpClientPermissionMode::Ask,
    })
}

pub(crate) fn migrate_legacy_builtin_client_configs(config_file: &mut AcpClientConfigFile) {
    for (client_id, legacy_args, current_args) in [
        ("claude-code", LEGACY_CLAUDE_ACP_ARGS, CLAUDE_ACP_ARGS),
        ("codex", LEGACY_CODEX_ACP_ARGS, CODEX_ACP_ARGS),
    ] {
        let Some(config) = config_file.acp_clients.get_mut(client_id) else {
            continue;
        };
        let uses_legacy_preset = config.command == "npx"
            && config
                .args
                .iter()
                .map(String::as_str)
                .eq(legacy_args.iter().copied());
        if uses_legacy_preset {
            config.args = current_args
                .iter()
                .map(|value| (*value).to_string())
                .collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_default_config_for_builtin_client() {
        let config = default_config_for_builtin_client("claude-code").expect("builtin config");
        assert!(config.enabled);
        assert_eq!(config.command, "npx");
        assert_eq!(
            config.args,
            vec!["--yes", "@agentclientprotocol/claude-agent-acp@latest"]
        );
    }

    #[test]
    fn omp_is_a_native_acp_preset() {
        let preset = builtin_acp_client_preset("omp").expect("omp preset registered");
        assert_eq!(preset.command, "omp");
        assert_eq!(preset.args, &["acp"]);
        assert_eq!(preset.tool_command, "omp");
        // Native ACP — no adapter package/bin, like opencode.
        assert!(preset.adapter_package.is_none());
        assert!(preset.adapter_bin.is_none());
        // User-managed: Alex provides no installer for omp.
        assert!(preset.install_package.is_none());

        let config = default_config_for_builtin_client("omp").expect("omp config");
        assert!(config.enabled);
        assert_eq!(config.command, "omp");
        assert_eq!(config.args, vec!["acp"]);
    }

    #[test]
    fn interpreter_is_a_native_acp_preset() {
        let preset =
            builtin_acp_client_preset("interpreter").expect("interpreter preset registered");
        assert_eq!(preset.command, "interpreter");
        assert_eq!(preset.args, &["acp"]);
        assert_eq!(preset.tool_command, "interpreter");
        // Native ACP — no adapter package/bin, like opencode and omp.
        assert!(preset.adapter_package.is_none());
        assert!(preset.adapter_bin.is_none());
        // User-managed: distributed as a standalone binary, not npm.
        assert!(preset.install_package.is_none());

        let config = default_config_for_builtin_client("interpreter").expect("interpreter config");
        assert!(config.enabled);
        assert_eq!(config.command, "interpreter");
        assert_eq!(config.args, vec!["acp"]);
    }

    #[test]
    fn dsh_preset_launches_the_alex_profile() {
        let preset = builtin_acp_client_preset("dsh").expect("dsh preset registered");
        assert_eq!(preset.command, "dsh");
        assert_eq!(preset.tool_command, "dsh");
        // The launch IS the profile: Alex ships the bridge, dsh runs it.
        assert_eq!(preset.args, &["--profile", DSH_BUNDLED_PROFILE]);
        assert_eq!(preset.bundled_profile, Some(DSH_BUNDLED_PROFILE));
        // Native ACP — the bridge is the profile, not a separate adapter.
        assert!(preset.adapter_package.is_none());
        assert!(preset.adapter_bin.is_none());
        // The harness itself is a plain npm global, so the installer applies.
        assert_eq!(preset.install_package, Some("@deepseek-ai/dsh"));

        // Every other preset is self-contained: nothing to materialize.
        for preset in BUILTIN_ACP_CLIENT_PRESETS.iter().filter(|p| p.id != "dsh") {
            assert!(
                preset.bundled_profile.is_none(),
                "{} needs no profile",
                preset.id
            );
        }
    }

    #[test]
    fn migrates_only_exact_legacy_builtin_commands() {
        let mut config_file = AcpClientConfigFile {
            acp_clients: HashMap::from([
                (
                    "claude-code".to_string(),
                    AcpClientConfig {
                        name: Some("Claude Code".to_string()),
                        command: "npx".to_string(),
                        args: LEGACY_CLAUDE_ACP_ARGS
                            .iter()
                            .map(|value| (*value).to_string())
                            .collect(),
                        env: HashMap::new(),
                        enabled: true,
                        readonly: false,
                        subagent: Default::default(),
                        permission_mode: AcpClientPermissionMode::Ask,
                    },
                ),
                (
                    "codex".to_string(),
                    AcpClientConfig {
                        name: Some("Codex".to_string()),
                        command: "npx".to_string(),
                        args: LEGACY_CODEX_ACP_ARGS
                            .iter()
                            .map(|value| (*value).to_string())
                            .collect(),
                        env: HashMap::new(),
                        enabled: true,
                        readonly: false,
                        subagent: Default::default(),
                        permission_mode: AcpClientPermissionMode::Ask,
                    },
                ),
                (
                    "custom-codex".to_string(),
                    AcpClientConfig {
                        name: Some("Pinned Codex".to_string()),
                        command: "npx".to_string(),
                        args: vec![
                            "--yes".to_string(),
                            "@zed-industries/codex-acp@0.16.0".to_string(),
                        ],
                        env: HashMap::new(),
                        enabled: true,
                        readonly: false,
                        subagent: Default::default(),
                        permission_mode: AcpClientPermissionMode::Ask,
                    },
                ),
            ]),
        };

        migrate_legacy_builtin_client_configs(&mut config_file);

        assert_eq!(
            config_file.acp_clients["claude-code"].args,
            vec!["--yes", "@agentclientprotocol/claude-agent-acp@latest"]
        );
        assert_eq!(
            config_file.acp_clients["codex"].args,
            vec!["--yes", "@agentclientprotocol/codex-acp@latest"]
        );
        assert_eq!(
            config_file.acp_clients["custom-codex"].args,
            vec!["--yes", "@zed-industries/codex-acp@0.16.0"]
        );
    }
}
