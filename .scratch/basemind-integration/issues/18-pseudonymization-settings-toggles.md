# 18: Pseudonymization settings toggles

**What to build:** **One** master switch in the Basemind settings page for pseudonymization control.

**Blocked by:** 14 (Basemind settings page)

**Status:** ready-for-agent

**Design authority:** corrected SPEC §11.8, decisions D2/D7. Re-scoped 2026-10-16 by D2 (`.scratch/pii-safe-rag-spine/issues/09-ticket-rescope-15-17-18.md`); the previous second switch (`pseudonymize_local_models`, "also pseudonymize for local models") is gone with ticket 15 — there is no model-type exemption, so there is nothing to toggle per model class.

- [ ] Extend `BasemindConfig` in Rust:

  ```rust
  pub struct BasemindConfig {
      // ... existing fields
      pub pseudonymize_prompts: bool, // master toggle — the ONLY pseudonymization switch, default true
  }
  ```

  **No** `pseudonymize_local_models` (D2: zero local-model exemptions). If a persisted
  config still carries that legacy key, deserialization must read-and-ignore it —
  never fail on it (upgrade compatibility).

- [ ] In `src/web-ui/src/infrastructure/config/components/RuntimeSettingsPages.tsx` Basemind section, a single switch:

  ```tsx
  <SettingRow>
    <SettingLabel>Pseudonymize user prompts</SettingLabel>
    <SettingDescription>
      Replace PII in your prompts before they reach the model — for every model,
      local ones included. Turning this off sends raw prompts to the selected
      model and voids the no-PII guarantee.
    </SettingDescription>
    <Switch
      checked={settings.pseudonymize_prompts}
      onChange={v => updateSettings({ pseudonymize_prompts: v })}
    />
  </SettingRow>
  ```

  Copy must not suggest local models are special in either direction — the guarantee
  (and the override) is model-agnostic (D2, SPEC §11.8).

- [ ] Defaults: `pseudonymize_prompts = true`.
- [ ] Wire to backend config persistence; ticket 17's interception consults exactly
  this one flag — every other failure path stays fail-closed (D7).
