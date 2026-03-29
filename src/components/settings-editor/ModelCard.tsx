import { useState } from 'react';
import { Cpu, X } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { useI18n } from '@/i18n/provider';

const PRESET_MODELS = [
  { value: 'claude-opus-4-6', label: 'Claude Opus 4.6' },
  { value: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6' },
  { value: 'claude-haiku-4-5', label: 'Claude Haiku 4.5' },
];

interface ModelCardProps {
  model: string;
  onChange: (model: string) => void;
}

export function ModelCard({ model, onChange }: ModelCardProps) {
  const { t } = useI18n();
  const [customInput, setCustomInput] = useState('');
  const isCustom = model !== '' && !PRESET_MODELS.some((p) => p.value === model);

  return (
    <Card>
      <CardHeader className="pb-4">
        <CardTitle className="flex items-center gap-2 text-base">
          <Cpu className="size-4" />
          {t('settingsEditor.model')}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => onChange('')}
            className={`rounded-lg border px-3 py-1.5 text-sm transition-all ${
              model === ''
                ? 'border-primary bg-primary/10 text-primary'
                : 'border-border text-muted-foreground hover:border-primary/30 hover:text-foreground'
            }`}
          >
            Default
          </button>
          {PRESET_MODELS.map((preset) => (
            <button
              key={preset.value}
              onClick={() => onChange(preset.value)}
              className={`rounded-lg border px-3 py-1.5 text-sm transition-all ${
                model === preset.value
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'border-border text-muted-foreground hover:border-primary/30 hover:text-foreground'
              }`}
            >
              {preset.label}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <Input
            value={isCustom ? model : customInput}
            onChange={(e) => {
              setCustomInput(e.target.value);
              if (e.target.value.trim()) onChange(e.target.value.trim());
            }}
            placeholder={t('settingsEditor.customModel')}
            className="h-8 text-sm font-mono"
          />
          {isCustom && (
            <Button variant="ghost" size="icon" className="size-8 shrink-0" onClick={() => { onChange(''); setCustomInput(''); }}>
              <X className="size-3.5" />
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
