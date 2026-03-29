import { useEffect, useState } from 'react';
import { Command as CommandIcon } from 'lucide-react';
import { getAppSetting, setAppSetting } from '@/lib/tauri-api';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { useI18n } from '@/i18n/provider';
import { cn } from '@/lib/utils';

function formatShortcut(shortcut: string) {
  return shortcut
    .replace(/Meta/i, '⌘')
    .replace(/Ctrl/i, '⌃')
    .replace(/Shift/i, '⇧')
    .replace(/Alt/i, '⌥')
    .replace(/\+/g, '')
    .replace(/([a-z])$/i, (match) => match.toUpperCase());
}

export function QuickLaunchCard() {
  const { t } = useI18n();
  const [paletteEnabled, setPaletteEnabled] = useState(false);
  const [paletteShortcut, setPaletteShortcut] = useState('Meta+k');
  const [recordingShortcut, setRecordingShortcut] = useState(false);

  useEffect(() => {
    getAppSetting('enable_command_palette').then(
      (value) => setPaletteEnabled(value === 'true')
    );
    getAppSetting('command_palette_shortcut').then(
      (value) => { if (value) setPaletteShortcut(value); }
    );
  }, []);

  return (
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
              await setAppSetting('enable_command_palette', enabled ? 'true' : 'false');
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
                  setAppSetting('command_palette_shortcut', value);
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
  );
}
