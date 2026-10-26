import { describe, expect, it } from 'vitest';
import { sessionSupportsThreadGoal } from './useComposerCapabilities';

describe('sessionSupportsThreadGoal', () => {
  it('supports Alex runtime primary agents, including Claw', () => {
    expect(sessionSupportsThreadGoal({ config: { agentType: 'Claw' }, mode: 'Claw' })).toBe(true);
  });

  it('does not advertise Alex thread goals for ACP-owned agents', () => {
    expect(
      sessionSupportsThreadGoal({ config: { agentType: 'acp:codex' }, mode: 'acp:codex' }),
    ).toBe(false);
    expect(sessionSupportsThreadGoal({ config: {}, mode: 'acp:claude-code' })).toBe(false);
  });
});
