import { useEffect, useState } from 'react';
import { Check, ChevronDown, ChevronRight, Code } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useI18n } from '@/i18n/provider';

interface AdvancedSettingsCardProps {
  advanced: Record<string, unknown>;
  onChange: (advanced: Record<string, unknown>) => void;
}

export function AdvancedSettingsCard({ advanced, onChange }: AdvancedSettingsCardProps) {
  const { t } = useI18n();
  const isEmpty = Object.keys(advanced).length === 0;
  const [expanded, setExpanded] = useState(!isEmpty);
  const [text, setText] = useState(() => JSON.stringify(advanced, null, 2));
  const [parseError, setParseError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    const newText = JSON.stringify(advanced, null, 2);
    if (!dirty) {
      setText(newText);
    }
  }, [advanced, dirty]);

  const handleChange = (value: string) => {
    setText(value);
    setDirty(true);
    try {
      JSON.parse(value);
      setParseError(null);
    } catch (e) {
      setParseError((e as Error).message);
    }
  };

  const handleSave = () => {
    try {
      const parsed = JSON.parse(text);
      if (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null) {
        setParseError(t('settingsEditor.mustBeObject'));
        return;
      }
      onChange(parsed);
      setDirty(false);
      setParseError(null);
    } catch (e) {
      setParseError((e as Error).message);
    }
  };

  return (
    <Card>
      <CardHeader className="pb-4">
        <button
          className="flex w-full items-center gap-2 text-left"
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
          <CardTitle className="flex items-center gap-2 text-base">
            <Code className="size-4" />
            {t('settingsEditor.advanced')}
          </CardTitle>
          {isEmpty && !expanded && (
            <span className="text-xs text-muted-foreground">{t('settingsEditor.advancedEmpty')}</span>
          )}
        </button>
      </CardHeader>
      {expanded && (
        <CardContent className="space-y-3">
          <textarea
            value={text}
            onChange={(e) => handleChange(e.target.value)}
            className="h-48 w-full resize-y rounded-lg border bg-muted/30 p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-primary/30"
            spellCheck={false}
          />
          {parseError && (
            <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
              {parseError}
            </div>
          )}
          {dirty && (
            <Button size="sm" onClick={handleSave} disabled={!!parseError}>
              <Check className="mr-1 size-3" />
              {t('settingsEditor.saveAdvanced')}
            </Button>
          )}
        </CardContent>
      )}
    </Card>
  );
}
