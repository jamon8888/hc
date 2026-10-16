# GLOSSARY

Canonical domain terms for this repository. Implementation-free: a definition here names a concept and its boundaries, not how it is built.

## Redaction

The **irreversible** removal of a PII value from text so the original cannot be recovered from that text. Applied wherever content crosses toward a model or leaves the local trust boundary — document conversion, tool results, prompt interception. Workspace files and the RAG index stay **inside** that boundary, in the same trust as the source files, and are not redacted at rest (ADR 0001, amended D6); what leaves them crosses at a chokepoint. Once redacted, the value is gone from that artifact; recovering it requires consulting a separate protected store — see Rehydration.

## Pseudonymization

The **reversible** substitution of a PII value with a stand-in token. Applied to everything crossing toward a model: user prompts first of all, and RAG hits on their way out. Unlike Redaction, the original value is retained in a protected store so the token can later be resolved back. The stand-in derives from Entity identity — an HMAC of the workspace Alias under a per-session secret — so the same entity is the same token within a session, a different token in every other session, and neither the Alias nor the session secret can be read off the token.

## Rehydration

The **reverse** of Pseudonymization: resolving a stand-in token back to its original PII value for display to the human user. Occurs only where content is rendered to the user; the model-facing transcript and session storage stay tokenized. (Workspace files and the RAG index are not rehydrated — under the source-file trust boundary they were never tokenized to begin with.)

## Entity identity

A stable identifier for a real-world PII entity — a person, an account number, an address — that persists across documents, prompts, and sessions within a workspace, so the same entity is recognizably the same entity wherever it appears. Distinct from a *redaction span* (which records **where** a value was found) and from a *token* (which is a session-scoped stand-in).

## Alias

The workspace-scoped stand-in form of an Entity identity. One real entity maps to exactly one Alias within a workspace, and to a distinct Alias in each other workspace.
