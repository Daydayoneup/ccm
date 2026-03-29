import { useEffect, useState } from 'react';
import { listEnvVars, setEnvVar, deleteEnvVar } from '@/lib/tauri-api';
import { Database, FolderOpen, Info, Settings, Variable } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { PageHeader, PageShell } from '@/components/layout/PageShell';
import { ApiSettingsCard } from '@/components/settings/ApiSettingsCard';
import { LanguageCard } from '@/components/settings/LanguageCard';
import { ProxyCard } from '@/components/settings/ProxyCard';
import { QuickLaunchCard } from '@/components/settings/QuickLaunchCard';
import { SyncCard } from '@/components/settings/SyncCard';
import { TerminalCard } from '@/components/settings/TerminalCard';
import { EnvVarTable } from '@/components/shared/EnvVarTable';
import { SettingsEditor } from '@/components/settings-editor/SettingsEditor';
import { useI18n } from '@/i18n/provider';
import type { MergedEnvVar } from '@/types/v2';

export function SettingsPage() {
  const { t } = useI18n();
  const [envVars, setEnvVars] = useState<MergedEnvVar[]>([]);

  useEffect(() => {
    listEnvVars(null)
      .then((vars) => setEnvVars(vars.map((item: any) => ({ ...item, scope: 'global' as const }))))
      .catch((error) => console.error('Failed to load env vars:', error));
  }, []);

  return (
    <PageShell className="gap-5">
      <PageHeader
        eyebrow={t('nav.settings')}
        title={t('settings.title')}
        description={t('settings.subtitle')}
      />

      <LanguageCard />
      <SyncCard />
      <ProxyCard />
      <ApiSettingsCard />
      <QuickLaunchCard />
      <TerminalCard />

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Settings className="size-5" />
            <CardTitle>{t('settingsEditor.globalTitle')}</CardTitle>
          </div>
          <CardDescription>{t('settingsEditor.globalDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <SettingsEditor project={null} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Variable className="size-5" />
            <CardTitle>{t('settings.environment')}</CardTitle>
          </div>
          <CardDescription>{t('settings.environmentDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <EnvVarTable
            vars={envVars}
            onAdd={async (key, value) => {
              await setEnvVar(null, key, value);
              const vars = await listEnvVars(null);
              setEnvVars(vars.map((item) => ({ ...item, scope: 'global' as const })));
            }}
            onDelete={async (id) => {
              await deleteEnvVar(id);
              const vars = await listEnvVars(null);
              setEnvVars(vars.map((item) => ({ ...item, scope: 'global' as const })));
            }}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Database className="size-5" />
            <CardTitle>{t('settings.database')}</CardTitle>
          </div>
          <CardDescription>{t('settings.databaseDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2">
            <FolderOpen className="size-4 text-muted-foreground" />
            <span className="text-sm font-mono text-muted-foreground">~/.claude-manager/ccm.db</span>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Info className="size-5" />
            <CardTitle>{t('settings.about')}</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div>
            <div className="text-sm font-medium">{t('settings.application')}</div>
            <div className="text-sm text-muted-foreground">{t('common.appSubtitle')}</div>
          </div>
          <Separator />
          <div>
            <div className="text-sm font-medium">{t('settings.version')}</div>
            <Badge variant="secondary">2.0.0</Badge>
          </div>
          <Separator />
          <div>
            <div className="text-sm font-medium">{t('settings.description')}</div>
            <div className="text-sm text-muted-foreground">{t('settings.appDescription')}</div>
          </div>
        </CardContent>
      </Card>
    </PageShell>
  );
}
