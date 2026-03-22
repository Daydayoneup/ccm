import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/button';
import {
  Card, CardHeader, CardTitle, CardDescription, CardContent,
} from '@/components/ui/card';
import { Switch } from '@/components/ui/switch';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Copy, RefreshCw, Wifi } from 'lucide-react';

export function ApiSettingsCard() {
  const [apiEnabled, setApiEnabled] = useState(false);
  const [apiPort, setApiPort] = useState('23890');
  const [tokenLast4, setTokenLast4] = useState<string | null>(null);
  const [newToken, setNewToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    invoke<string | null>('get_app_setting', { key: 'api_enabled' }).then(
      (val) => setApiEnabled(val === 'true')
    );
    invoke<string | null>('get_app_setting', { key: 'api_port' }).then(
      (val) => { if (val) setApiPort(val); }
    );
    invoke<string | null>('get_api_token_status').then(setTokenLast4);
  }, []);

  const handleToggle = async (enabled: boolean) => {
    setApiEnabled(enabled);
    await invoke('toggle_api_server', { enabled });
  };

  const handlePortChange = async (port: string) => {
    setApiPort(port);
    const num = parseInt(port, 10);
    if (num >= 1024 && num <= 65535) {
      await invoke('set_app_setting', { key: 'api_port', value: port });
    }
  };

  const handleGenerateToken = async () => {
    const token = await invoke<string>('generate_api_token');
    setNewToken(token);
    setTokenLast4(token.slice(-4));
    setCopied(false);
  };

  const handleCopy = () => {
    if (newToken) {
      navigator.clipboard.writeText(newToken);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Wifi className="size-5" />
            <CardTitle>HTTP API</CardTitle>
          </div>
          <Switch checked={apiEnabled} onCheckedChange={handleToggle} />
        </div>
        <CardDescription>
          Local HTTP API for external tools (Raycast, Alfred, scripts)
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>Port</Label>
          <Input
            className="w-32"
            value={apiPort}
            onChange={(e) => handlePortChange(e.target.value)}
            disabled={apiEnabled}
            placeholder="23890"
          />
        </div>

        <div className="space-y-2">
          <Label>API Token</Label>
          <div className="flex items-center gap-2">
            {newToken ? (
              <>
                <code className="flex-1 rounded border bg-muted px-3 py-2 text-sm font-mono break-all">
                  {newToken}
                </code>
                <Button size="sm" variant="outline" onClick={handleCopy}>
                  <Copy className="mr-1 size-4" />
                  {copied ? 'Copied!' : 'Copy'}
                </Button>
              </>
            ) : tokenLast4 ? (
              <span className="text-sm text-muted-foreground font-mono">
                ****...{tokenLast4}
              </span>
            ) : (
              <span className="text-sm text-muted-foreground">No token generated</span>
            )}
          </div>
          <Button
            size="sm"
            variant={tokenLast4 ? 'outline' : 'default'}
            onClick={handleGenerateToken}
            disabled={!apiEnabled}
          >
            <RefreshCw className="mr-1 size-4" />
            {tokenLast4 ? 'Regenerate Token' : 'Generate Token'}
          </Button>
          {newToken && (
            <p className="text-xs text-amber-600 dark:text-amber-400">
              Copy this token now — it won't be shown again.
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
