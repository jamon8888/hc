import { beforeAll, describe, expect, it } from 'vitest';
import { pinI18nLocale } from '@/test/i18nTestUtils';
import {
  resolveThreadGoalHeaderTitle,
  resolveThreadGoalUserMessageDisplay,
} from './threadGoalDisplay';

// The raw thread-goal text is English, so only a catalog in another language
// can prove that the display came from the catalog. Pin the catalog locale
// instead of inheriting the product default.
beforeAll(async () => {
  await pinI18nLocale('zh-CN');
});

describe('resolveThreadGoalUserMessageDisplay', () => {
  it('localizes objective-updated metadata', () => {
    const result = resolveThreadGoalUserMessageDisplay('Thread goal updated: foo', {
      threadGoalObjectiveUpdated: true,
      objective: '同步上游',
    });
    expect(result).toContain('同步上游');
    expect(result).not.toContain('Thread goal updated');
  });

  it('localizes auto-continuation completion-check metadata', () => {
    const result = resolveThreadGoalUserMessageDisplay('Continuing thread goal: sync', {
      threadGoalContinuation: true,
      objective: '同步上游',
      autoContinuationAttempt: 2,
      autoContinuationMax: 100,
    });
    expect(result).toContain('同步上游');
    expect(result).not.toContain('Continuing thread goal');
    expect(result).toContain('目标完成检查');
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
