# 17: Interception in desktop_dialog_turn_request()

**What to build:** An **unconditional** pseudonymization branch in `apps/desktop/src/api/agentic_api.rs` `desktop_dialog_turn_request()` (D2 — there is no cloud/local split), plus history rehydration on session restore.

**Blocked by:** 16 (PromptPseudonymizer)

**Status:** ready-for-agent

**Design authority:** corrected SPEC §11.4/§11.7, decisions D2/D4/D7. Re-scoped 2026-10-16 by D2 (`.scratch/pii-safe-rag-spine/issues/09-ticket-rescope-15-17-18.md`); ticket 15 (local model detection) is superseded and is no longer a blocker.

- [ ] Modify `desktop_dialog_turn_request()` signature to accept `state: &AppState` (if not already)
- [ ] Add the interception — this runs for **every** model, local included:

  ```rust
  #[cfg(feature = "basemind")]
  let pseudonymized = if state.basemind_config().pseudonymize_prompts {
      // The master toggle (§11.8) is the ONLY skip condition.
      basemind_integration::get_prompt_pseudonymizer(&state)?
          .pseudonymize(&request.user_input, &request.session_id)
          .await
          .map_err(|e| format!("PII pseudonymization failed (basemind required): {e}"))?
  } else {
      // Explicit user override; voids I9 and says so in Settings (§11.9).
      PseudonymizedPrompt::raw(request.user_input.clone())
  };

  Ok(AgentDialogTurnRequest {
      session_id: request.session_id,
      message: pseudonymized.text,                  // session tokens only
      original_message: Some(request.user_input),   // UI display only (D4)
      metadata,                                     // unchanged — see rules below
      // ...
  })
  ```

- [ ] **No model-type branch (D2):** there is no `should_pseudonymize`, no
  `is_local_model_base_url`, no `get_model_base_url` lookup. Model provenance never
  enters the decision.
- [ ] **Fail closed (D7):** with the master toggle on, basemind unreachable / registry
  unreadable / vault unwritable ⇒ the turn aborts with a clear error. Silent fallback
  to the raw prompt is forbidden (only an explicit `pseudonymize_prompts = false`
  bypasses pseudonymization at all).
- [ ] **Never put the rehydration map in metadata.** The original body's
  `metadata.insert("prompt_rehydration", …)` stored `token → value` in persisted
  request metadata — a D4 leak. The map lives only in the session vault; the returned
  `rehydration_map` is for internal handoff and must not be serialized into request,
  event, or model-visible fields.
- [ ] Preserve `original_user_input` for UI display (`original_message`).
- [ ] Session restore (`restore_session_with_turns`): rehydrate the **display
  projection** only —

  ```rust
  #[cfg(feature = "basemind")]
  {
      let pseudonymizer = basemind_integration::get_prompt_pseudonymizer(&state)?;
      for turn in turns.iter_mut() {
          if let Some(user_msg) = turn.user_message.as_mut() {
              user_msg.content = pseudonymizer.rehydrate(&user_msg.content, &session_id).await
                  .unwrap_or_else(|_| user_msg.content.clone());
          }
      }
  }
  ```

  The stored transcript stays tokenized — it is re-sent to the model as-is (D4).
- [ ] Live display of RAG hits and tool results resolves tokens through the same
  `rehydrate()` at the UI render boundary — not in metadata, not in storage.
- [ ] Registry `flush()` and vault store stay inside `pseudonymize()` (ticket 16) —
  this ticket only branches and forwards.
