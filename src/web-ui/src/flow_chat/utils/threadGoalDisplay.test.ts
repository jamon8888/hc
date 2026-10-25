import { describe, expect, it } from 'vitest';
import { i18nService } from '@/infrastructure/i18n/core/I18nService';
import {
  resolveThreadGoalHeaderTitle,
  resolveThreadGoalUserMessageDisplay,
} from './threadGoalDisplay';

// The raw thread-goal text is whatever the model streamed, so it is never the
// same string as a catalog entry. The assertions below therefore hold in every
// locale, and the expected copy is read from the active catalog instead of
// being pinned to one language.
const RAW_OBJECTIVE_UPDATED = 'raw-thread-goal-payload';
const RAW_CONTINUATION = 'raw-thread-goal-continuation-payload';

describe('resolveThreadGoalUserMessageDisplay', () => {
  it('localizes objective-updated metadata', () => {
    const result = resolveThreadGoalUserMessageDisplay(RAW_OBJECTIVE_UPDATED, {
      threadGoalObjectiveUpdated: true,
      objective: '同步上游',
    });
    expect(result).toContain('同步上游');
    expect(result).not.toContain(RAW_OBJECTIVE_UPDATED);
    expect(result).toBe(
      i18nService.t('flow-chat:threadGoal.updatedUserMessage', { objective: '同步上游' }),
    );
  });

  it('localizes auto-continuation completion-check metadata', () => {
    const result = resolveThreadGoalUserMessageDisplay(RAW_CONTINUATION, {
      threadGoalContinuation: true,
      objective: '同步上游',
      autoContinuationAttempt: 2,
      autoContinuationMax: 100,
    });
    expect(result).toContain('同步上游');
    expect(result).not.toContain(RAW_CONTINUATION);
    expect(result).toBe(
      i18nService.t('flow-chat:threadGoal.continuationCheckUserMessage', { objective: '同步上游' }),
    );
  });
});

describe('resolveThreadGoalHeaderTitle', () => {
  it('returns completion-check header for continuation turns', () => {
    const title = resolveThreadGoalHeaderTitle({
      threadGoalContinuation: true,
      autoContinuationAttempt: 3,
      autoContinuationMax: 100,
    });
    expect(title).toBeTruthy();
    expect(title).toContain('3');
  });
});
