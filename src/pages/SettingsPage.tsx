import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Database, FolderOpen, Info, Terminal, Variable, Globe, Loader2, ChevronDown, ChevronUp, Check, X, Command as CommandIcon } from 'lucide-react';
import { Switch } from '@/components/ui/switch';
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
import { cn } from '@/lib/utils';
import { useSyncStore } from '@/stores/sync-store';
import { EnvVarTable } from '@/components/shared/EnvVarTable';
import type { MergedEnvVar } from '@/types/v2';
import { ApiSettingsCard } from '@/components/settings/ApiSettingsCard';

export function SettingsPage() {
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
  }, []);

  useEffect(() => {
    invoke<string | null>('get_app_setting', { key: 'enable_command_palette' }).then(
      (val) => setPaletteEnabled(val === 'true')
    );
    invoke<string | null>('get_app_setting', { key: 'command_palette_shortcut' }).then(
      (val) => { if (val) setPaletteShortcut(val); }
    );
  }, []);

  useEffect(() => {
    loadEnvVars();
  }, []);

  useEffect(() => {
    invoke<any>('get_proxy_config').then((config) => {
      if (config) {
        setProxyEnabled(config.enabled);
        setProxyType(config.proxy_type);
        setProxyHost(config.host);
        setProxyPort(config.port);
        setProxyUsername(config.username ?? '');
        setProxyPassword(config.password ?? '');
        if (config.username || config.password) setShowAuth(true);
      }
    });
  }, []);

  const loadEnvVars = async () => {
    try {
      const vars = await invoke<any[]>('list_env_vars', { projectId: null });
      setEnvVars(vars.map((v) => ({ ...v, scope: 'global' as const })));
    } catch (err) {
      console.error('Failed to load env vars:', err);
    }
  };

  const handleAddEnvVar = async (key: string, value: string) => {
    await invoke('set_env_var', { projectId: null, key, value });
    await loadEnvVars();
  };

  const handleDeleteEnvVar = async (id: string) => {
    await invoke('delete_env_var', { id });
    await loadEnvVars();
  };

  const handleTerminalChange = async (value: string) => {
    setTerminal(value);
    await invoke('set_terminal_preference', { terminal: value });
  };

  const handleSaveProxy = async () => {
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
    } catch (err) {
      console.error('Failed to save proxy config:', err);
    } finally {
      setProxySaving(false);
    }
  };

  const handleTestProxy = async () => {
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
    } catch (err) {
      setProxyTestResult({ success: false, message: String(err) });
    } finally {
      setProxyTesting(false);
    }
  };

  const handlePaletteToggle = async (enabled: boolean) => {
    setPaletteEnabled(enabled);
    await invoke('set_app_setting', { key: 'enable_command_palette', value: enabled ? 'true' : 'false' });
    window.dispatchEvent(new Event('settings-changed'));
  };

  const handleShortcutRecord = (e: React.KeyboardEvent) => {
    if (!recordingShortcut) return;
    e.preventDefault();
    const parts: string[] = [];
    if (e.metaKey) parts.push('Meta');
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.shiftKey) parts.push('Shift');
    if (e.altKey) parts.push('Alt');
    if (['Meta', 'Control', 'Shift', 'Alt'].includes(e.key)) return;
    parts.push(e.key.toLowerCase());
    const shortcutVal = parts.join('+');
    setPaletteShortcut(shortcutVal);
    setRecordingShortcut(false);
    invoke('set_app_setting', { key: 'command_palette_shortcut', value: shortcutVal });
    window.dispatchEvent(new Event('settings-changed'));
  };

  const formatShortcut = (s: string) => {
    return s
      .replace(/Meta/i, '⌘')
      .replace(/Ctrl/i, '⌃')
      .replace(/Shift/i, '⇧')
      .replace(/Alt/i, '⌥')
      .replace(/\+/g, '')
      .replace(/([a-z])$/i, (m) => m.toUpperCase());
  };

  const handleSync = async () => {
    try {
      const result = await invoke<{ status: string }>('full_sync');
      if (result.status === 'queued') {
        // Already running, queued for re-run — UI will update via events
      }
    } catch (err) {
      console.error('Failed to trigger sync:', err);
    }
  };

  return (
    <div className="space-y-6 p-6">
      <div>
        <h1 className="text-2xl font-bold">Settings</h1>
        <p className="text-sm text-muted-foreground">
          Application configuration and maintenance
        </p>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <RefreshCw className="size-5" />
            <CardTitle>Sync</CardTitle>
          </div>
          <CardDescription>
            Synchronize project data and resource indexes
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4">
            <Button onClick={handleSync} disabled={syncStatus === 'queued'} size="sm">
              <RefreshCw
                className={`mr-2 size-4 ${syncStatus === 'running' ? 'animate-spin' : ''}`}
              />
              {syncStatus === 'running'
                ? 'Syncing...'
                : syncStatus === 'queued'
                  ? 'Queued...'
                  : 'Run Full Sync'}
            </Button>
            {lastSynced && (
              <span className="text-sm text-muted-foreground">
                Last synced: {lastSynced}
              </span>
            )}
          </div>
          {syncStatus === 'running' && syncProgress && (
            <div className="rounded-md border border-blue-500/30 bg-blue-500/10 p-3 text-sm text-blue-700 dark:text-blue-400">
              {syncProgress.message}
            </div>
          )}
          {syncError && (
            <div className="rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
              Sync failed: {syncError}
            </div>
          )}
          {syncStatus === 'idle' && !syncError && lastSynced && (
            <div className="rounded-md border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-700 dark:text-green-400">
              Sync completed successfully.
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Globe className="size-5" />
              <CardTitle>Network Proxy</CardTitle>
            </div>
            <Switch checked={proxyEnabled} onCheckedChange={setProxyEnabled} />
          </div>
          <CardDescription>
            Configure proxy for git operations (registry sync)
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-[1fr_200px_1fr] gap-4">
            <div className="space-y-2">
              <Label>Type</Label>
              <Select value={proxyType} onValueChange={setProxyType} disabled={!proxyEnabled}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="http">HTTP/HTTPS</SelectItem>
                  <SelectItem value="socks5">SOCKS5</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2 col-span-1">
              <Label>Host</Label>
              <Input
                placeholder="127.0.0.1"
                value={proxyHost}
                onChange={(e) => setProxyHost(e.target.value)}
                disabled={!proxyEnabled}
              />
            </div>
            <div className="space-y-2">
              <Label>Port</Label>
              <Input
                placeholder="7890"
                value={proxyPort}
                onChange={(e) => setProxyPort(e.target.value)}
                disabled={!proxyEnabled}
              />
            </div>
          </div>

          <button
            type="button"
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => setShowAuth(!showAuth)}
            disabled={!proxyEnabled}
          >
            {showAuth ? <ChevronUp className="size-4" /> : <ChevronDown className="size-4" />}
            Authentication (optional)
          </button>

          {showAuth && (
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Username</Label>
                <Input
                  placeholder="username"
                  value={proxyUsername}
                  onChange={(e) => setProxyUsername(e.target.value)}
                  disabled={!proxyEnabled}
                />
              </div>
              <div className="space-y-2">
                <Label>Password</Label>
                <Input
                  type="password"
                  placeholder="password"
                  value={proxyPassword}
                  onChange={(e) => setProxyPassword(e.target.value)}
                  disabled={!proxyEnabled}
                />
              </div>
            </div>
          )}

          <div className="flex items-center gap-3">
            <Button size="sm" onClick={handleSaveProxy} disabled={!proxyEnabled || proxySaving}>
              {proxySaving ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}
              Save
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={handleTestProxy}
              disabled={!proxyEnabled || proxyTesting || !proxyHost || !proxyPort}
            >
              {proxyTesting ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}
              Test Connection
            </Button>
            {proxySaved && (
              <span className="flex items-center gap-1 text-sm text-green-600">
                <Check className="size-4" /> Saved
              </span>
            )}
          </div>

          {proxyTestResult && (
            <div
              className={`rounded-md border p-3 text-sm ${
                proxyTestResult.success
                  ? 'border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-400'
                  : 'border-destructive bg-destructive/10 text-destructive'
              }`}
            >
              {proxyTestResult.success ? <Check className="inline size-4 mr-1" /> : <X className="inline size-4 mr-1" />}
              {proxyTestResult.message}
            </div>
          )}
        </CardContent>
      </Card>

      <ApiSettingsCard />

      {/* Quick Launch */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <CommandIcon className="size-4 text-primary" />
            <CardTitle className="text-base">Quick Launch</CardTitle>
          </div>
          <CardDescription>
            Command Palette for fast project search and shell launch
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="palette-toggle">Enable Command Palette</Label>
            <Switch
              id="palette-toggle"
              checked={paletteEnabled}
              onCheckedChange={handlePaletteToggle}
            />
          </div>
          {paletteEnabled && (
            <div className="space-y-2">
              <Label>Shortcut</Label>
              <div className="flex items-center gap-2">
                <div
                  tabIndex={0}
                  onKeyDown={handleShortcutRecord}
                  onFocus={() => setRecordingShortcut(true)}
                  onBlur={() => setRecordingShortcut(false)}
                  className={cn(
                    'flex h-9 w-32 items-center justify-center rounded-md border text-sm font-mono cursor-pointer transition-colors',
                    recordingShortcut
                      ? 'border-primary bg-primary/5 text-primary'
                      : 'bg-muted text-foreground'
                  )}
                >
                  {recordingShortcut ? 'Press keys...' : formatShortcut(paletteShortcut)}
                </div>
                <span className="text-xs text-muted-foreground">Click to change</span>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Terminal className="size-5" />
            <CardTitle>Terminal</CardTitle>
          </div>
          <CardDescription>
            Choose which terminal app to use when launching Claude
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Select value={terminal} onValueChange={handleTerminalChange}>
            <SelectTrigger className="w-48">
              <SelectValue />
            </SelectTrigger>
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
            <CardTitle>Claude Environment</CardTitle>
          </div>
          <CardDescription>
            Environment variables passed to Claude CLI on launch (global)
          </CardDescription>
        </CardHeader>
        <CardContent>
          <EnvVarTable
            vars={envVars}
            onAdd={handleAddEnvVar}
            onDelete={handleDeleteEnvVar}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Database className="size-5" />
            <CardTitle>Database</CardTitle>
          </div>
          <CardDescription>Data storage information</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2">
            <FolderOpen className="size-4 text-muted-foreground" />
            <span className="text-sm font-mono text-muted-foreground">
              ~/.claude-manager/ccm.db
            </span>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Info className="size-5" />
            <CardTitle>About</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div>
            <div className="text-sm font-medium">Application</div>
            <div className="text-sm text-muted-foreground">
              Claude Config Manager (CCM)
            </div>
          </div>
          <Separator />
          <div>
            <div className="text-sm font-medium">Version</div>
            <Badge variant="secondary">2.0.0</Badge>
          </div>
          <Separator />
          <div>
            <div className="text-sm font-medium">Description</div>
            <div className="text-sm text-muted-foreground">
              Manage Claude Code resources across projects
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
