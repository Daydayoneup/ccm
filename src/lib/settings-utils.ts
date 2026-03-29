export interface SettingsPermissions {
  allow: string[];
  ask: string[];
  deny: string[];
}

export interface SplitSettings {
  permissions: SettingsPermissions;
  model: string;
  env: Record<string, string>;
  advanced: Record<string, unknown>;
}

/** Split a raw settings.json object into structured card data + remaining advanced fields */
export function splitSettings(raw: Record<string, unknown>): SplitSettings {
  const { permissions: rawPerm, model: rawModel, env: rawEnv, ...advanced } = raw;

  const perm = (rawPerm ?? {}) as Record<string, unknown>;
  const permissions: SettingsPermissions = {
    allow: Array.isArray(perm.allow) ? (perm.allow as string[]) : [],
    ask: Array.isArray(perm.ask) ? (perm.ask as string[]) : [],
    deny: Array.isArray(perm.deny) ? (perm.deny as string[]) : [],
  };

  const model = typeof rawModel === 'string' ? rawModel : '';
  const env = (rawEnv != null && typeof rawEnv === 'object' && !Array.isArray(rawEnv))
    ? (rawEnv as Record<string, string>)
    : {};

  return { permissions, model, env, advanced };
}

/** Merge structured card data + advanced fields back into a single settings object */
export function mergeSettings(split: SplitSettings): Record<string, unknown> {
  const result: Record<string, unknown> = { ...split.advanced };

  // Only write permissions if at least one rule exists
  const { allow, ask, deny } = split.permissions;
  if (allow.length > 0 || ask.length > 0 || deny.length > 0) {
    const permissions: Record<string, string[]> = {};
    if (allow.length > 0) permissions.allow = allow;
    if (ask.length > 0) permissions.ask = ask;
    if (deny.length > 0) permissions.deny = deny;
    result.permissions = permissions;
  }

  // Only write model if non-empty
  if (split.model) {
    result.model = split.model;
  }

  // Only write env if non-empty
  if (Object.keys(split.env).length > 0) {
    result.env = split.env;
  }

  return result;
}

export type SettingsScope = 'shared' | 'local';

/** Compute the file path for a given project + scope combination */
export function getSettingsPath(projectPath: string, scope: SettingsScope): string {
  if (scope === 'shared') {
    return `${projectPath}/.claude/settings.json`;
  }
  return `${projectPath}/.claude/settings.local.json`;
}

/** Path for global settings */
export const GLOBAL_SETTINGS_PATH = '~/.claude/settings.json';
