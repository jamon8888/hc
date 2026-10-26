import { describe, expect, it } from 'vitest';
import { formatSkillDetailPath } from './skillDetailPath';

describe('Skill detail path presentation', () => {
  it('keeps complete directories around the omission in the reported Windows path', () => {
    expect(formatSkillDetailPath(String.raw`C:\Users\HUAWEI\AppData\Roaming\alex\skills\.system\gstack-autoplan`))
      .toBe(String.raw`C:\Users\HUAWEI\…\.system\gstack-autoplan`);
  });

  it('preserves a remote POSIX root and complete trailing directory names', () => {
    expect(formatSkillDetailPath('/home/alex/.config/alex/skills/.system/create-alex-skin'))
      .toBe('/home/alex/.config/…/.system/create-alex-skin');
  });

  it('preserves the UNC prefix and trailing separator', () => {
    expect(formatSkillDetailPath('\\\\server\\share\\team\\tools\\alex\\skills\\gstack-autoplan\\'))
      .toBe('\\\\server\\share\\team\\…\\skills\\gstack-autoplan\\');
  });

  it.each(['', '/', 'C:\\', 'C:\\skills\\gstack-autoplan', '/home/alex/skills/gstack-autoplan'])(
    'leaves a short path unchanged: %s',
    path => expect(formatSkillDetailPath(path)).toBe(path),
  );
});
