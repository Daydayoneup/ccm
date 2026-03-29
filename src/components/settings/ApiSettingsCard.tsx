import { useEffect, useState } from 'react';
import { getAppSetting, toggleApiServer, setAppSetting, getApiTokenStatus, generateApiToken } from '@/lib/tauri-api';
import { Copy, RefreshCw, Wifi } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { useI18n } from '@/i18n/provider';

export function ApiSettingsCard() {
  const { t } = useI18n();
  const [apiEnabled, setApiEnabled] = useState(false);
  const [apiPort, setApiPort] = useState('23890');
  const [tokenLast4, setTokenLast4] = useState<string | null>(null);
  const [newToken, setNewToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    getAppSetting('api_enabled').then(
      (value) => setApiEnabled(value === 'true')
    );
    getAppSetting('api_port').then(
      (value) => { if (value) setApiPort(value); }
    );
    getApiTokenStatus().then(setTokenLast4);
  }, []);

  const handleToggle = async (enabled: boolean) => {
    setApiEnabled(enabled);
    await toggleApiServer(enabled);
  };

  const handlePortChange = async (port: string) => {
    setApiPort(port);
    const num = parseInt(port, 10);
    if (num >= 1024 && num <= 65535) {
      await setAppSetting('api_port', port);
    }
  };

  const handleGenerateToken = async () => {
    const token = await generateApiToken();
    setNewToken(token);
    setTokenLast4(token.slice(-4));
    setCopied(false);
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Wifi className="size-5" />
            <CardTitle>{t('settings.httpApi')}</CardTitle>
          </div>
          <Switch checked={apiEnabled} onCheckedChange={handleToggle} />
        </div>
        <CardDescription>{t('settings.httpApiDescription')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>{t('settings.apiPort')}</Label>
          <Input
            className="w-32"
            value={apiPort}
            onChange={(e) => handlePortChange(e.target.value)}
            disabled={apiEnabled}
            placeholder="23890"
          />
        </div>

        <div className="space-y-2">
          <Label>{t('settings.apiToken')}</Label>
          <div className="flex items-center gap-2">
            {newToken ? (
              <>
                <code className="flex-1 break-all rounded border bg-muted px-3 py-2 text-sm font-mono">
                  {newToken}
                </code>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    navigator.clipboard.writeText(newToken);
                    setCopied(true);
                    setTimeout(() => setCopied(false), 2000);
                  }}
                >
                  <Copy className="mr-1 size-4" />
                  {copied ? t('settings.copied') : t('settings.copy')}
                </Button>
              </>
            ) : tokenLast4 ? (
              <span className="font-mono text-sm text-muted-foreground">****...{tokenLast4}</span>
            ) : (
              <span className="text-sm text-muted-foreground">{t('settings.noToken')}</span>
            )}
          </div>
          <Button
            size="sm"
            variant={tokenLast4 ? 'outline' : 'default'}
            onClick={handleGenerateToken}
            disabled={!apiEnabled}
          >
            <RefreshCw className="mr-1 size-4" />
            {tokenLast4 ? t('settings.regenerateToken') : t('settings.generateToken')}
          </Button>
          {newToken ? (
            <p className="text-xs text-amber-600 dark:text-amber-400">{t('settings.tokenHint')}</p>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}
