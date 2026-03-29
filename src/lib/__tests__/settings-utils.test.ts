import { describe, it, expect } from 'vitest';
import { splitSettings, mergeSettings, getSettingsPath } from '../settings-utils';

describe('splitSettings', () => {
  it('splits a full settings object', () => {
    const raw = {
      permissions: { allow: ['Bash(git *)'], ask: ['Bash(git push *)'], deny: ['Bash(rm -rf *)'] },
      model: 'claude-opus-4-6',
      env: { FOO: 'bar' },
      hooks: { PreToolUse: [] },
      statusLine: { type: 'command', command: 'echo hi' },
    };
    const result = splitSettings(raw);
    expect(result.permissions).toEqual({ allow: ['Bash(git *)'], ask: ['Bash(git push *)'], deny: ['Bash(rm -rf *)'] });
    expect(result.model).toBe('claude-opus-4-6');
    expect(result.env).toEqual({ FOO: 'bar' });
    expect(result.advanced).toEqual({ hooks: { PreToolUse: [] }, statusLine: { type: 'command', command: 'echo hi' } });
  });

  it('handles empty object', () => {
    const result = splitSettings({});
    expect(result.permissions).toEqual({ allow: [], ask: [], deny: [] });
    expect(result.model).toBe('');
    expect(result.env).toEqual({});
    expect(result.advanced).toEqual({});
  });

  it('handles missing ask field in permissions', () => {
    const raw = { permissions: { allow: ['Read(*)'], deny: [] } };
    const result = splitSettings(raw);
    expect(result.permissions.ask).toEqual([]);
  });
});

describe('mergeSettings', () => {
  it('merges back into a flat object', () => {
    const split = {
      permissions: { allow: ['Bash(git *)'], ask: [], deny: ['Bash(rm *)'] },
      model: 'claude-opus-4-6',
      env: { KEY: 'val' },
      advanced: { hooks: {} },
    };
    const result = mergeSettings(split);
    expect(result.permissions).toEqual({ allow: ['Bash(git *)'], deny: ['Bash(rm *)'] });
    expect(result.model).toBe('claude-opus-4-6');
    expect(result.env).toEqual({ KEY: 'val' });
    expect(result.hooks).toEqual({});
  });

  it('omits empty permissions/model/env', () => {
    const split = {
      permissions: { allow: [], ask: [], deny: [] },
      model: '',
      env: {},
      advanced: { hooks: {} },
    };
    const result = mergeSettings(split);
    expect(result.permissions).toBeUndefined();
    expect(result.model).toBeUndefined();
    expect(result.env).toBeUndefined();
    expect(result.hooks).toEqual({});
  });

  it('structured fields override advanced if duplicated', () => {
    const split = {
      permissions: { allow: ['new'], ask: [], deny: [] },
      model: 'new-model',
      env: {},
      advanced: { permissions: { allow: ['old'] }, model: 'old-model' },
    };
    const result = mergeSettings(split);
    expect(result.permissions).toEqual({ allow: ['new'] });
    expect(result.model).toBe('new-model');
  });
});

describe('getSettingsPath', () => {
  it('returns shared path', () => {
    expect(getSettingsPath('/home/user/project', 'shared')).toBe('/home/user/project/.claude/settings.json');
  });
  it('returns local path', () => {
    expect(getSettingsPath('/home/user/project', 'local')).toBe('/home/user/project/.claude/settings.local.json');
  });
});
