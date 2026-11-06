import { describe, expect, it, vi } from 'vitest';
import { workspaceAPI } from './WorkspaceAPI';
import { createWordcraftWorkspaceSaver, WordcraftDocumentHost } from './WordcraftDocumentHost';

const sessionId = 'wordcraft-session-1';

describe('WordcraftDocumentHost', () => {
  it('persists returned bytes and acknowledges a successful save', async () => {
    const saveBytes = vi.fn().mockResolvedValue(undefined);
    const acknowledgeSave = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave });

    host.handleEvent({
      type: 'save',
      requestId: 'save-1',
      sessionId,
      revision: 4,
      name: 'draft.docx',
      bytes: new Uint8Array([80, 75, 3, 4]),
    });

    await vi.waitFor(() => expect(saveBytes).toHaveBeenCalledWith(new Uint8Array([80, 75, 3, 4]), 4));
    expect(acknowledgeSave).toHaveBeenCalledWith('save-1', 4, true);
    expect(host.state).toMatchObject({ dirty: false, revision: 4, saving: false, error: undefined });
  });

  it('resolves durable-save waiters only after workspace persistence completes', async () => {
    let persist!: () => void;
    const saveBytes = vi.fn(() => new Promise<void>(resolve => { persist = resolve; }));
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave: vi.fn() });
    let settled = false;
    const durable = host.waitForDurableSave(4).then(() => { settled = true; });

    host.handleEvent({
      type: 'save',
      requestId: 'save-durable',
      sessionId,
      revision: 4,
      name: 'draft.docx',
      bytes: new Uint8Array([4]),
    });
    await Promise.resolve();
    expect(settled).toBe(false);

    persist();
    await durable;
    expect(settled).toBe(true);
  });

  it('surfaces save failures and sends a negative acknowledgement', async () => {
    const saveBytes = vi.fn().mockRejectedValue(new Error('workspace is offline'));
    const acknowledgeSave = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave }, new Uint8Array([9]));

    host.handleEvent({
      type: 'save',
      requestId: 'save-2',
      sessionId,
      revision: 5,
      name: 'draft.docx',
      bytes: new Uint8Array([1, 2, 3]),
    });

    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith('save-2', 5, false, 'workspace is offline'));
    expect(host.state).toMatchObject({ dirty: true, revision: 5, saving: false, error: 'workspace is offline' });
    expect(host.getLatestBytes()).toEqual(new Uint8Array([9]));
  });

  it('rejects empty save payloads before persistence', async () => {
    const saveBytes = vi.fn().mockResolvedValue(undefined);
    const acknowledgeSave = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave });

    host.handleEvent({
      type: 'save',
      requestId: 'save-empty',
      sessionId,
      revision: 6,
      name: 'draft.docx',
      bytes: new Uint8Array(),
    });

    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith(
      'save-empty',
      6,
      false,
      'WordCraft save payload must be a non-empty binary buffer',
    ));
    expect(saveBytes).not.toHaveBeenCalled();
    expect(host.state).toMatchObject({ dirty: true, revision: 6, saving: false });
  });

  it('continues processing saves when an acknowledgement callback throws', async () => {
    const saveBytes = vi.fn().mockResolvedValue(undefined);
    const acknowledgeSave = vi.fn()
      .mockImplementationOnce(() => { throw new Error('iframe detached'); });
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave });

    host.handleEvent({
      type: 'save', requestId: 'save-ack-1', sessionId, revision: 1, name: 'draft.docx', bytes: new Uint8Array([1]),
    });
    await vi.waitFor(() => expect(saveBytes).toHaveBeenCalledTimes(1));

    host.handleEvent({
      type: 'save', requestId: 'save-ack-2', sessionId, revision: 2, name: 'draft.docx', bytes: new Uint8Array([2]),
    });
    await vi.waitFor(() => expect(saveBytes).toHaveBeenCalledTimes(2));

    expect(acknowledgeSave).toHaveBeenCalledWith('save-ack-2', 2, true);
    expect(host.state).toMatchObject({ dirty: false, revision: 2, saving: false, error: undefined });
  });

  it('keeps newer dirty state and saving status while an older save completes', async () => {
    let resolveFirst!: () => void;
    let resolveSecond!: () => void;
    const saveBytes = vi.fn()
      .mockImplementationOnce(() => new Promise<void>(resolve => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise<void>(resolve => { resolveSecond = resolve; }));
    const acknowledgeSave = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave });

    host.handleEvent({
      type: 'save', requestId: 'save-4', sessionId, revision: 4, name: 'draft.docx', bytes: new Uint8Array([4]),
    });
    host.handleEvent({
      type: 'save', requestId: 'save-5', sessionId, revision: 5, name: 'draft.docx', bytes: new Uint8Array([5]),
    });

    await vi.waitFor(() => expect(saveBytes).toHaveBeenCalledTimes(1));
    resolveFirst();
    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith('save-4', 4, true));
    await vi.waitFor(() => expect(saveBytes).toHaveBeenCalledTimes(2));
    expect(host.state).toMatchObject({ dirty: true, revision: 5, saving: true });

    resolveSecond();
    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith('save-5', 5, true));
    expect(host.state).toMatchObject({ dirty: false, revision: 5, saving: false, error: undefined });
  });

  it('drains accepted saves before disposal', async () => {
    let resolveFirst!: () => void;
    let resolveSecond!: () => void;
    const saveBytes = vi.fn()
      .mockImplementationOnce(() => new Promise<void>(resolve => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise<void>(resolve => { resolveSecond = resolve; }));
    const acknowledgeSave = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave });

    host.handleEvent({ type: 'save', requestId: 'save-dispose-1', sessionId, revision: 1, name: 'draft.docx', bytes: new Uint8Array([1]) });
    host.handleEvent({ type: 'save', requestId: 'save-dispose-2', sessionId, revision: 2, name: 'draft.docx', bytes: new Uint8Array([2]) });
    host.dispose();

    resolveFirst();
    await vi.waitFor(() => expect(saveBytes).toHaveBeenCalledTimes(2));
    expect(acknowledgeSave).toHaveBeenCalledWith('save-dispose-1', 1, true);
    resolveSecond();
    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith('save-dispose-2', 2, true));
    expect(host.state).toMatchObject({ saving: false, dirty: false, revision: 2 });
  });

  it('rejects a save event older than the latest accepted revision', async () => {
    const saveBytes = vi.fn().mockResolvedValue(undefined);
    const acknowledgeSave = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave });

    host.handleEvent({
      type: 'save', requestId: 'save-5', sessionId, revision: 5, name: 'draft.docx', bytes: new Uint8Array([5]),
    });
    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith('save-5', 5, true));

    host.handleEvent({
      type: 'save', requestId: 'save-4', sessionId, revision: 4, name: 'draft.docx', bytes: new Uint8Array([4]),
    });

    expect(acknowledgeSave).toHaveBeenCalledWith(
      'save-4',
      4,
      false,
      'WordCraft save revision is stale',
    );
    expect(saveBytes).toHaveBeenCalledTimes(1);
    expect(host.state).toMatchObject({ dirty: false, revision: 5, saving: false });
  });

  it('keeps newer dirty state and saving status while an older save completes', async () => {
    let resolveFirst!: () => void;
    let resolveSecond!: () => void;
    const saveBytes = vi.fn()
      .mockImplementationOnce(() => new Promise<void>(resolve => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise<void>(resolve => { resolveSecond = resolve; }));
    const acknowledgeSave = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes, acknowledgeSave });

    host.handleEvent({
      type: 'save', requestId: 'save-4', sessionId, revision: 4, name: 'draft.docx', bytes: new Uint8Array([4]),
    });
    host.handleEvent({
      type: 'save', requestId: 'save-5', sessionId, revision: 5, name: 'draft.docx', bytes: new Uint8Array([5]),
    });

    await vi.waitFor(() => expect(saveBytes).toHaveBeenCalledTimes(1));
    resolveFirst();
    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith('save-4', 4, true));
    expect(host.state).toMatchObject({ dirty: true, revision: 5, saving: true });

    resolveSecond();
    await vi.waitFor(() => expect(acknowledgeSave).toHaveBeenCalledWith('save-5', 5, true));
    expect(host.state).toMatchObject({ dirty: false, revision: 5, saving: false, error: undefined });
  });

  it('tracks dirty state and ignores stale sessions and events after teardown', () => {
    const onState = vi.fn();
    const host = new WordcraftDocumentHost({ sessionId, saveBytes: vi.fn(), acknowledgeSave: vi.fn(), onState });

    host.handleEvent({ type: 'dirty', sessionId, revision: 2, dirty: true });
    expect(host.state).toMatchObject({ dirty: true, revision: 2 });

    host.handleEvent({ type: 'dirty', sessionId: 'stale-session', revision: 99, dirty: false });
    expect(host.state.revision).toBe(2);

    host.dispose();
    host.handleEvent({ type: 'dirty', sessionId, revision: 3, dirty: false });
    expect(host.state.revision).toBe(2);
    expect(onState).toHaveBeenCalled();
  });

  it('resolves the current workspace hash for each save', async () => {
    const write = vi.spyOn(workspaceAPI, 'writeWorkspaceFileBase64').mockResolvedValue(undefined);
    const getExpectedHash = vi.fn().mockReturnValueOnce('hash-before-first').mockReturnValueOnce('hash-before-second');
    const save = createWordcraftWorkspaceSaver('workspace-1', '/docs/draft.docx', getExpectedHash);

    await save(new Uint8Array([1, 2, 3]), 1);
    await save(new Uint8Array([4, 5, 6]), 2);

    expect(write).toHaveBeenNthCalledWith(1, 'workspace-1', '/docs/draft.docx', 'AQID', 'hash-before-first');
    expect(write).toHaveBeenNthCalledWith(2, 'workspace-1', '/docs/draft.docx', 'BAUG', 'hash-before-second');
    write.mockRestore();
  });
});
