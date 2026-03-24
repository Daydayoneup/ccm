import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Check,
  ChevronDown,
  ChevronUp,
  Command as CommandIcon,
  Database,
  FolderOpen,
  Globe,
  Info,
  Loader2,
  RefreshCw,
  Terminal,
  Variable,
  X,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { PageHeader, PageShell } from '@/components/layout/PageShell';
import { ApiSettingsCard } from '@/components/settings/ApiSettingsCard';
import { EnvVarTable } from '@/components/shared/EnvVarTable';
import { useI18n, type Locale } from '@/i18n/provider';
import { cn } from '@/lib/utils';
import { useSyncStore } from '@/stores/sync-store';
import type { MergedEnvVar } from '@/types/v2';

export function SettingsPage() {
  const { t, locale, setLocale } = useI18n();
  const { syncStatus, syncProgress, syncError, lastSynced } = useSyncStore();
  const [terminal, setTerminal] = useState('Terminal');
  const [paletteEnabled, setPaletteEnabled] = useState(false);
  const [paletteShortcut, setPaletteShortcut] = useState('Meta+k');
  const [recordingShortcut, setRecordingShortcut] = useState(false);
  const [envVars, setEnvVars] = useState<MergedEnvVar[]>([]);
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [proxyType, setProxyType] = useState('http');
  const [proxyHost, setProxyHost] = useState('');
  const [proxyPort, setProxyPort] = useState('');
  const [proxyUsername, setProxyUsername] = useState('');
  const [proxyPassword, setProxyPassword] = useState('');
  const [showAuth, setShowAuth] = useState(false);
  const [proxySaving, setProxySaving] = useState(false);
  const [proxyTesting, setProxyTesting] = useState(false);
  const [proxyTestResult, setProxyTestResult] = useState<{ success: boolean; message: string } | null>(null);
  const [proxySaved, setProxySaved] = useState(false);

  useEffect(() => {
    invoke<string>('get_terminal_preference').then(setTerminal);
    invoke<string | null>('get_app_setting', { key: 'enable_command_palette' }).then(
      (value) => setPaletteEnabled(value === 'true')
    );
    invoke<string | null>('get_app_setting', { key: 'command_palette_shortcut' }).then(
      (value) => { if (value) setPaletteShortcut(value); }
    );
    invoke<any>('get_proxy_config').then((config) => {
      if (!config) return;
      setProxyEnabled(config.enabled);
      setProxyType(config.proxy_type);
      setProxyHost(config.host);
      setProxyPort(config.port);
      setProxyUsername(config.username ?? '');
      setProxyPassword(config.password ?? '');
      if (config.username || config.password) setShowAuth(true);
    });
    invoke<any[]>('list_env_vars', { projectId: null })
      .then((vars) => setEnvVars(vars.map((item) => ({ ...item, scope: 'global' as const }))))
      .catch((error) => console.error('Failed to load env vars:', error));
  }, []);

  const formatShortcut = (shortcut: string) => shortcut
    .replace(/Meta/i, '⌘')
    .replace(/Ctrl/i, '⌃')
    .replace(/Shift/i, '⇧')
    .replace(/Alt/i, '⌥')
    .replace(/\+/g, '')
    .replace(/([a-z])$/i, (match) => match.toUpperCase());

  return (
    <PageShell className="gap-5">
      <PageHeader
        eyebrow={t('nav.settings')}
        title={t('settings.title')}
        description={t('settings.subtitle')}
      />

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Globe className="size-5" />
            <CardTitle>{t('settings.language')}</CardTitle>
          </div>
          <CardDescription>{t('settings.languageDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <Select value={locale} onValueChange={(value) => setLocale(value as Locale)}>
            <SelectTrigger className="w-56">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="system">{t('settings.localeSystem')}</SelectItem>
              <SelectItem value="zh-CN">{t('settings.localeZhCN')}</SelectItem>
              <SelectItem value="en">{t('settings.localeEn')}</SelectItem>
            </SelectContent>
          </Select>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <RefreshCw className="size-5" />
            <CardTitle>{t('settings.sync')}</CardTitle>
          </div>
          <CardDescription>{t('settings.syncDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4">
            <Button
              onClick={async () => {
                try {
                  await invoke('full_sync');
                } catch (error) {
                  console.error('Failed to trigger sync:', error);
                }
              }}
              disabled={syncStatus === 'queued'}
              size="sm"
            >
              <RefreshCw className={`mr-2 size-4 ${syncStatus === 'running' ? 'animate-spin' : ''}`} />
              {syncStatus === 'running'
                ? t('common.syncing')
                : syncStatus === 'queued'
                  ? t('settings.queued')
                  : t('settings.runFullSync')}
            </Button>
            {lastSynced ? <span className="text-sm text-muted-foreground">{t('settings.lastSynced', { value: lastSynced })}</span> : null}
          </div>
          {syncStatus === 'running' && syncProgress ? (
            <div className="rounded-md border border-blue-500/30 bg-blue-500/10 p-3 text-sm text-blue-700 dark:text-blue-400">
              {syncProgress.message}
            </div>
          ) : null}
          {syncError ? (
            <div className="rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
              {t('settings.syncFailed', { message: syncError })}
            </div>
          ) : null}
          {syncStatus === 'idle' && !syncError && lastSynced ? (
            <div className="rounded-md border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-700 dark:text-green-400">
              {t('settings.syncSuccess')}
            </div>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Globe className="size-5" />
              <CardTitle>{t('settings.proxy')}</CardTitle>
            </div>
            <Switch checked={proxyEnabled} onCheckedChange={setProxyEnabled} />
          </div>
          <CardDescription>{t('settings.proxyDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-[1fr_200px_1fr] gap-4">
            <div className="space-y-2">
              <Label>{t('settings.proxyType')}</Label>
              <Select value={proxyType} onValueChange={setProxyType} disabled={!proxyEnabled}>
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="http">HTTP/HTTPS</SelectItem>
                  <SelectItem value="socks5">SOCKS5</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>{t('settings.proxyHost')}</Label>
              <Input placeholder="127.0.0.1" value={proxyHost} onChange={(e) => setProxyHost(e.target.value)} disabled={!proxyEnabled} />
            </div>
            <div className="space-y-2">
              <Label>{t('settings.proxyPort')}</Label>
              <Input placeholder="7890" value={proxyPort} onChange={(e) => setProxyPort(e.target.value)} disabled={!proxyEnabled} />
            </div>
          </div>

          <button
            type="button"
            className="flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
            onClick={() => setShowAuth(!showAuth)}
            disabled={!proxyEnabled}
          >
            {showAuth ? <ChevronUp className="size-4" /> : <ChevronDown className="size-4" />}
            {t('settings.proxyAuth')}
          </button>

          {showAuth ? (
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>{t('settings.proxyUsername')}</Label>
                <Input placeholder="username" value={proxyUsername} onChange={(e) => setProxyUsername(e.target.value)} disabled={!proxyEnabled} />
              </div>
              <div className="space-y-2">
                <Label>{t('settings.proxyPassword')}</Label>
                <Input type="password" placeholder="password" value={proxyPassword} onChange={(e) => setProxyPassword(e.target.value)} disabled={!proxyEnabled} />
              </div>
            </div>
          ) : null}

          <div className="flex items-center gap-3">
            <Button
              size="sm"
              onClick={async () => {
                setProxySaving(true);
                setProxySaved(false);
                try {
                  await invoke('save_proxy_config', {
                    config: {
                      enabled: proxyEnabled,
                      proxy_type: proxyType,
                      host: proxyHost,
                      port: proxyPort,
                      username: proxyUsername || null,
                      password: proxyPassword || null,
                    },
                  });
                  setProxySaved(true);
                  setTimeout(() => setProxySaved(false), 3000);
                } finally {
                  setProxySaving(false);
                }
              }}
              disabled={!proxyEnabled || proxySaving}
            >
              {proxySaving ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}
              {t('common.save')}
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={async () => {
                setProxyTesting(true);
                setProxyTestResult(null);
                try {
                  await invoke('save_proxy_config', {
                    config: {
                      enabled: true,
                      proxy_type: proxyType,
                      host: proxyHost,
                      port: proxyPort,
                      username: proxyUsername || null,
                      password: proxyPassword || null,
                    },
                  });
                  const message = await invoke<string>('test_proxy');
                  setProxyTestResult({ success: true, message });
                } catch (error) {
                  setProxyTestResult({ success: false, message: String(error) });
                } finally {
                  setProxyTesting(false);
                }
              }}
              disabled={!proxyEnabled || proxyTesting || !proxyHost || !proxyPort}
            >
              {proxyTesting ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}
              {t('settings.testConnection')}
            </Button>
            {proxySaved ? (
              <span className="flex items-center gap-1 text-sm text-green-600">
                <Check className="size-4" />
                {t('settings.saved')}
              </span>
            ) : null}
          </div>

          {proxyTestResult ? (
            <div
              className={`rounded-md border p-3 text-sm ${
                proxyTestResult.success
                  ? 'border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-400'
                  : 'border-destructive bg-destructive/10 text-destructive'
              }`}
            >
              {proxyTestResult.success ? <Check className="mr-1 inline size-4" /> : <X className="mr-1 inline size-4" />}
              {proxyTestResult.message}
            </div>
          ) : null}
        </CardContent>
      </Card>

      <ApiSettingsCard />

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <CommandIcon className="size-4 text-primary" />
            <CardTitle className="text-base">{t('settings.quickLaunch')}</CardTitle>
          </div>
          <CardDescription>{t('settings.quickLaunchDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="palette-toggle">{t('settings.enableCommandPalette')}</Label>
            <Switch
              id="palette-toggle"
              checked={paletteEnabled}
              onCheckedChange={async (enabled) => {
                setPaletteEnabled(enabled);
                await invoke('set_app_setting', { key: 'enable_command_palette', value: enabled ? 'true' : 'false' });
                window.dispatchEvent(new Event('settings-changed'));
              }}
            />
          </div>
          {paletteEnabled ? (
            <div className="space-y-2">
              <Label>{t('settings.shortcut')}</Label>
              <div className="flex items-center gap-2">
                <div
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (!recordingShortcut) return;
                    e.preventDefault();
                    const parts: string[] = [];
                    if (e.metaKey) parts.push('Meta');
                    if (e.ctrlKey) parts.push('Ctrl');
                    if (e.shiftKey) parts.push('Shift');
                    if (e.altKey) parts.push('Alt');
                    if (['Meta', 'Control', 'Shift', 'Alt'].includes(e.key)) return;
                    parts.push(e.key.toLowerCase());
                    const value = parts.join('+');
                    setPaletteShortcut(value);
                    setRecordingShortcut(false);
                    invoke('set_app_setting', { key: 'command_palette_shortcut', value });
                    window.dispatchEvent(new Event('settings-changed'));
                  }}
                  onFocus={() => setRecordingShortcut(true)}
                  onBlur={() => setRecordingShortcut(false)}
                  className={cn(
                    'flex h-9 w-32 cursor-pointer items-center justify-center rounded-md border text-sm font-mono transition-colors',
                    recordingShortcut ? 'border-primary bg-primary/5 text-primary' : 'bg-muted text-foreground'
                  )}
                >
                  {recordingShortcut ? t('settings.pressKeys') : formatShortcut(paletteShortcut)}
                </div>
                <span className="text-xs text-muted-foreground">{t('settings.clickToChange')}</span>
              </div>
            </div>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Terminal className="size-5" />
            <CardTitle>{t('settings.terminal')}</CardTitle>
          </div>
          <CardDescription>{t('settings.terminalDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <Select
            value={terminal}
            onValueChange={async (value) => {
              setTerminal(value);
              await invoke('set_terminal_preference', { terminal: value });
            }}
          >
            <SelectTrigger className="w-48"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="Terminal">Terminal.app</SelectItem>
              <SelectItem value="iTerm2">iTerm2</SelectItem>
              <SelectItem value="Warp">Warp</SelectItem>
            </SelectContent>
          </Select>
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
              await invoke('set_env_var', { projectId: null, key, value });
              const vars = await invoke<any[]>('list_env_vars', { projectId: null });
              setEnvVars(vars.map((item) => ({ ...item, scope: 'global' as const })));
            }}
            onDelete={async (id) => {
              await invoke('delete_env_var', { id });
              const vars = await invoke<any[]>('list_env_vars', { projectId: null });
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
