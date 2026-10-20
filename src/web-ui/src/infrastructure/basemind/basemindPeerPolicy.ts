/**
 * Whether the Basemind redaction surface may run for a workspace.
 *
 * The Product Operation Registry marks `basemind_read_redacted_document` and
 * `basemind_reveal_entity` as controller-local: the controller runs them and
 * they read the controller's own Basemind vault. A workspace that lives on a
 * connected device is therefore not redactable here — the controller would
 * answer with its own files instead of the peer's — so peer-owned workspaces
 * get an explicit unsupported state instead of a wrong document.
 */
import { getActiveSurfaceId, isLocalSurface } from '@/infrastructure/peer-device/deviceSurface';
import { workspaceManager } from '@/infrastructure/services/business/workspaceManager';

/**
 * Whether the Basemind surface must refuse the given workspace.
 *
 * The workspace ID selects the document; the owning device surface decides
 * which machine hosts it. An unknown workspace ID falls back to the rendered
 * surface, because the rendered surface is the only host the caller can see.
 */
export function isBasemindPeerRefused(workspaceId: string | undefined): boolean {
  const id = workspaceId?.trim() ?? '';
  if (!id) {
    return !isLocalSurface(getActiveSurfaceId());
  }
  const owner = workspaceManager.surfaceOwningWorkspace(id);
  if (owner === null) {
    return !isLocalSurface(getActiveSurfaceId());
  }
  return !isLocalSurface(owner);
}
