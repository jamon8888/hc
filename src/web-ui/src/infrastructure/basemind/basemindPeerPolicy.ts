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
import {
  getActiveSurfaceId,
  isLocalSurface,
  type DeviceSurfaceId,
} from '@/infrastructure/peer-device/deviceSurface';
import { workspaceManager } from '@/infrastructure/services/business/workspaceManager';

/**
 * Whether the Basemind surface must refuse the given workspace.
 *
 * The resource surface is authoritative: it names the device that owns the
 * document, and the same workspace ID can exist on two devices. The
 * workspace-ID lookup is only a fallback for callers that do not carry a
 * surface; it prefers the rendered surface and therefore cannot resolve a
 * peer resource to the local surface when the caller knows better.
 */
export function isBasemindPeerRefused(
  workspaceId: string | undefined,
  surfaceId?: DeviceSurfaceId,
): boolean {
  if (surfaceId) {
    return !isLocalSurface(surfaceId);
  }
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
