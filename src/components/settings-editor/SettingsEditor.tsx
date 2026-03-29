import { useCallback, useEffect, useState } from 'react';
import { readSettingsFile, writeSettingsFile } from '@/lib/tauri-api';
import { splitSettings, mergeSettings, getSettingsPath, GLOBAL_SETTINGS_PATH } from '@/lib/settings-utils';
import type { SplitSettings, SettingsScope } from '@/lib/settings-utils';
import type { Project } from '@/types/v2';
import { ScopeToggle } from './ScopeToggle';
import { PermissionsCard } from './PermissionsCard';
import { ModelCard } from './ModelCard';
import { EnvVarsCard } from './EnvVarsCard';
import { AdvancedSettingsCard } from './AdvancedSettingsCard';
import { useI18n } from '@/i18n/provider';

interface SettingsEditorProps {
  project: Project | null;
}

export function SettingsEditor({ project }: SettingsEditorProps) {
  const { t } = useI18n();
  const [scope, setScope] = useState<SettingsScope>('shared');
  const [settings, setSettings] = useState<SplitSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const filePath = project
    ? getSettingsPath(project.path, scope)
    : GLOBAL_SETTINGS_PATH;

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const raw = await readSettingsFile(filePath);
      setSettings(splitSettings(raw as Record<string, unknown>));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [filePath]);

  useEffect(() => { load(); }, [load]);

  const save = async (updated: SplitSettings) => {
    setSaving(true);
    setError(null);
    try {
      const merged = mergeSettings(updated);
      await writeSettingsFile(filePath, merged);
      setSettings(updated);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12 text-muted-foreground">
        {t('settingsEditor.loading')}
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="flex items-center justify-center py-12 text-muted-foreground">
        {error ?? t('settingsEditor.loadError')}
      </div>
    );
  }

  return (
    <div className="space-y-5">
      {project && (
        <ScopeToggle scope={scope} onScopeChange={setScope} filePath={filePath} />
      )}

      {error && <div className="rounded-lg border border-destructive bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
      {saving && <div className="text-xs text-muted-foreground">{t('settingsEditor.saving')}</div>}

      <PermissionsCard
        permissions={settings.permissions}
        onChange={(permissions) => save({ ...settings, permissions })}
      />
      <ModelCard
        model={settings.model}
        onChange={(model) => save({ ...settings, model })}
      />
      <EnvVarsCard
        env={settings.env}
        onChange={(env) => save({ ...settings, env })}
      />
      <AdvancedSettingsCard
        advanced={settings.advanced}
        onChange={(advanced) => save({ ...settings, advanced })}
      />
    </div>
  );
}
