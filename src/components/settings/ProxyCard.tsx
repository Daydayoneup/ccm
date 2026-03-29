import { useEffect, useState } from 'react';
import {
  getProxyConfig, saveProxyConfig, testProxy,
  type ProxyConfig,
} from '@/lib/tauri-api';
import { Check, ChevronDown, ChevronUp, Globe, Loader2, X } from 'lucide-react';
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
import { Switch } from '@/components/ui/switch';
import { useI18n } from '@/i18n/provider';

export function ProxyCard() {
  const { t } = useI18n();
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
    getProxyConfig().then((config) => {
      if (!config) return;
      setProxyEnabled(config.enabled);
      setProxyType(config.proxy_type);
      setProxyHost(config.host);
      setProxyPort(config.port);
      setProxyUsername(config.username ?? '');
      setProxyPassword(config.password ?? '');
      if (config.username || config.password) setShowAuth(true);
    });
  }, []);

  const buildConfig = (): ProxyConfig => ({
    enabled: proxyEnabled,
    proxy_type: proxyType,
    host: proxyHost,
    port: proxyPort,
    username: proxyUsername || null,
    password: proxyPassword || null,
  });

  const handleSave = async () => {
    setProxySaving(true);
    setProxySaved(false);
    try {
      await saveProxyConfig(buildConfig());
      setProxySaved(true);
      setTimeout(() => setProxySaved(false), 3000);
    } finally {
      setProxySaving(false);
    }
  };

  const handleTest = async () => {
    setProxyTesting(true);
    setProxyTestResult(null);
    try {
      await saveProxyConfig({ ...buildConfig(), enabled: true });
      const message = await testProxy({});
      setProxyTestResult({ success: true, message });
    } catch (error) {
      setProxyTestResult({ success: false, message: String(error) });
    } finally {
      setProxyTesting(false);
    }
  };

  return (
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
          <Button size="sm" onClick={handleSave} disabled={!proxyEnabled || proxySaving}>
            {proxySaving ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}
            {t('common.save')}
          </Button>
          <Button size="sm" variant="outline" onClick={handleTest} disabled={!proxyEnabled || proxyTesting || !proxyHost || !proxyPort}>
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
  );
}
