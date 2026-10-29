# 14: Basemind config schema + Settings page

**What to build:** Configuration schema in Rust and Settings UI page in Web UI for Basemind options.

**Blocked by:** 13 (model management)

**Status:** ready-for-agent

- [ ] In `alex_core::service::config::types` (or `basemind-integration`):
  ```rust
  #[derive(Serialize, Deserialize, Clone, Default)]
  pub struct BasemindConfig {
      pub enabled: bool,
      pub auto_download_models: bool,
      pub model_directory: Option<PathBuf>,
      pub redact_filenames: bool,           // default false
      pub models: HashMap<String, ModelConfig>,
      pub sidecar_path: Option<PathBuf>,
  }
  
  #[derive(Serialize, Deserialize, Clone)]
  pub struct ModelConfig {
      pub version: String,
      pub enabled: bool,
      pub local_path: Option<PathBuf>,
  }
  ```
- [ ] In `src/web-ui/src/infrastructure/config/components/RuntimeSettingsPages.tsx`:
  - Add "Basemind" settings section
  - Master toggle: "Enable Basemind document redaction"
  - Toggle: "Auto-download models on first use"
  - Toggle: "Redact filenames in file listings" (default off)
  - Model table: name, version, status, actions (Download/Remove)
  - Advanced: custom sidecar path, model directory
  - Remote: "Use remote Basemind on SSH host"
- [ ] Persist in `GlobalConfig` under `basemind` key