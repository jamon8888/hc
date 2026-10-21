# 0001: Own the PII entity-identity layer in the product, not the basemind fork

Status: accepted

## Context

OpenBitFun integrates basemind as a sidecar for PII detection, redaction, indexing, and retrieval. The product guarantee is that no PII reaches any LLM — cloud *or* local — while the user's view stays fully rehydrated, including across turns ("Jean Dupont's chronology").

basemind v0.33.0 provides **no cross-document PII identity**:

- `entity_id` is a per-file span id: `format!("{file_id}:{category}:{start}:{end}")` (`src/pii/translation.rs:84`).
- `value_hash` is a hash of the *positional redaction token*, not of the value: `hash_bytes(f.token)` where token is e.g. `[IBAN_1]` (`src/pii/translation.rs:85`).
- `pii_lineage` (`src/index/pii_lineage.rs`) is a GDPR Art. 30 audit trail of *where* PII was found — not a *who/what-is-this* identity graph.
- Search hits (`Bm25Hit`, `FusedHit`, `LexicalHit`, `DocumentHit`, …) carry no entity references.

So the cross-turn, cross-document correlation the product requires cannot be built on the fork's primitives. The original `SPEC-basemind-integration.md` assumed it could (§4 declares `entity_ids` on hits; §11.5 minted positional `[PERSON_1]` tokens), which is why the spec contradicted itself.

## Decision

The entity-identity layer is **product logic owned by `basemind-integration`** in this repository, not an extension of the basemind fork. `hc` orchestrates the PII pipeline — detect → build value → alias registry → redact → vault → index — and treats basemind as an adapter providing detection, indexing/retrieval, and vault storage.

The registry is:

- **per-workspace** — the same real entity gets a distinct alias in different workspaces, so no cross-workspace correlation is possible;
- **a new, dedicated, vault-encrypted store** — not the fork's `pii_lineage` keyspace, keeping the process boundary clean;
- keyed by a hash of the **real value**, mapped to a stable alias; the per-session salt is applied to that alias only at the LLM boundary, never persisted into the index.

## Consequences

- The fork stays clean and low-divergence; all PII-identity policy lives where the product owns it, per the repository rule to keep product logic in the product and expose basemind through an adapter.
- `hc` must see the plaintext value at redaction time to hash it. The value is never stored raw — only its hash and the vault-encrypted token → value map (the vault key lives in the OS keychain, passed over IPC, never argv).
- Adds a new persistent store and an orchestration path the original spec did not have.
- If basemind is unavailable the turn **hard-fails**; a PII guarantee that silently degrades when a sidecar hiccups is not a guarantee.

## Alternatives rejected

- **(B) Extend the fork with a value-keyed identity primitive.** Coupling the product to fork internals across a process boundary already chosen as clean (D5), and grows fork divergence that must be maintained on every upgrade.
- **(C) Accept positional tokens and drop cross-document correlation.** Fails the stated product requirement — the LLM could not be told "this is the same Jean as before", and RAG hits could not be joined to prompts.

## Amendments (2026-10-16)

Refinements settled while writing the corrected spec §4/§11. They change how the
decision is expressed, not what it decides; D1–D10 stay locked.

- **The RAG index sits inside the source-file trust boundary (D6 amended).** The index
  is raw. basemind's scan daemon ingests workspace files directly and exposes no
  "index now" RPC (evidence ticket 01), so an alias-substituted index was never
  achievable without forking the fork — and it is not wanted: the registry's aliases
  and the session salt are never written into the index, and `[documents].redaction.enabled`
  is set to `false` so basemind's own positional tokens stay out of it too. The
  pipeline listed under *Decision* therefore reads `detect → alias registry → session
  token at every boundary (LLM, remote)` — indexing is basemind's own raw path, not a
  step `hc` feeds. What must never happen is raw text *crossing* a boundary, not raw
  bytes *at rest*.
- **The session token is opaque, not decomposable (D3 clarified).** The token is
  `[<CATEGORY>_<H16>]`, `H16` = first 16 hex characters of
  `HMAC-SHA256(key = session salt, message = workspace alias)`. Neither the alias nor
  the salt appears in the token; a token reverses only through the session vault's
  `token → value` map. This rejects the earlier `alias + salt` concatenation, which
  exposed the salt as a plaintext suffix — an observer of model context could strip
  it, re-derive the workspace-stable alias, link the entity across sessions of the
  workspace, and confirm a guessed value. Cross-session unlinkability now rests on the
  hidden salt; within-session stability on the fixed `(salt, alias)` pair.
- **Exposure seam.** The layer's outward face is `RagSearchPort`
  (`contracts/runtime-ports`, feature `rag-search`): `openbitfun-core` implements the
  agent `Tool` against the trait and is built **without** any dependency on
  `basemind-integration`; the concrete port is injected at the host composition root —
  same pattern as `RemoteExecPort`. This keeps the repository rule intact: product
  logic lives in the product crate, exposed through a stable port, with basemind
  behind an adapter.

**Version note.** Evidence (ADR context, tickets 01–02) was gathered against
`jamon8888/basemind` **v0.33.0**; the pin is now **v0.34.0** (2026-10-19), recorded in
code as `openbitfun_basemind_integration::BASEMIND_VERSION`. v0.34.0 is the upstream sync
of `Goldziher/basemind` v0.29.3 into the fork (1,202 commits). The sync keeps the fork's
PII, redaction and vault features end to end — the alternative this ADR rejected is still
rejected, so the fork still offers no value-keyed identity primitive and the identity layer
remains product logic. The upstream `Goldziher/basemind` v0.29.x line is a diverged
numbering stream released after our tag and is not the integration target.

**Upgrade note.** v0.34.0 advances the fork's release minor from 33 to 34, and the
persisted blob, index and git-cache schema versions derive from it. The first scan after
upgrading therefore resets each view in place and rebuilds it once. A workspace indexed by
v0.33.x re-indexes on the first scan against v0.34.0; no product state is lost, because
the identity registry and the session vault are product-owned and carry their own schema.
