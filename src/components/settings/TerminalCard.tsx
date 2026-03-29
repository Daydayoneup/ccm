import { useEffect, useState } from 'react';
import { Terminal } from 'lucide-react';
import { getTerminalPreference, setTerminalPreference } from '@/lib/tauri-api';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useI18n } from '@/i18n/provider';

export function TerminalCard() {
  const { t } = useI18n();
  const [terminal, setTerminal] = useState('Terminal');

  useEffect(() => {
    getTerminalPreference().then((v) => { if (v) setTerminal(v); });
  }, []);

  return (
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
            await setTerminalPreference(value);
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
  );
}
