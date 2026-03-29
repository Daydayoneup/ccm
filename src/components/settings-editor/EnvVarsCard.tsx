import { useState } from 'react';
import { Check, Eye, EyeOff, Plus, Trash2, Variable, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { useI18n } from '@/i18n/provider';

interface EnvVarsCardProps {
  env: Record<string, string>;
  onChange: (env: Record<string, string>) => void;
}

export function EnvVarsCard({ env, onChange }: EnvVarsCardProps) {
  const { t } = useI18n();
  const [adding, setAdding] = useState(false);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [visibleKeys, setVisibleKeys] = useState<Set<string>>(new Set());

  const entries = Object.entries(env);

  const toggleVisibility = (key: string) => {
    setVisibleKeys((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const handleAdd = () => {
    const k = newKey.trim();
    const v = newValue.trim();
    if (!k || !v) return;
    onChange({ ...env, [k]: v });
    setNewKey('');
    setNewValue('');
    setAdding(false);
  };

  const handleDelete = (key: string) => {
    const next = { ...env };
    delete next[key];
    onChange(next);
  };

  return (
    <Card>
      <CardHeader className="pb-4">
        <CardTitle className="flex items-center gap-2 text-base">
          <Variable className="size-4" />
          {t('settingsEditor.envVars')}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="rounded-[20px] border">
          {entries.length === 0 && !adding ? (
            <div className="px-4 py-6 text-center text-xs text-muted-foreground">
              {t('settingsEditor.noEnvVars')}
            </div>
          ) : (
            <div className="divide-y">
              {entries.map(([key, value]) => (
                <div key={key} className="flex items-center gap-3 px-4 py-2.5">
                  <code className="min-w-[140px] text-sm font-semibold">{key}</code>
                  <code className="flex-1 text-sm text-muted-foreground">
                    {visibleKeys.has(key) ? value : '••••••••'}
                  </code>
                  <div className="flex items-center gap-0.5">
                    <Button variant="ghost" size="icon" className="size-7" onClick={() => toggleVisibility(key)}>
                      {visibleKeys.has(key) ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
                    </Button>
                    <Button variant="ghost" size="icon" className="size-7" onClick={() => handleDelete(key)}>
                      <Trash2 className="size-3.5" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
          {adding && (
            <div className="flex items-center gap-2 border-t px-3 py-2">
              <Input value={newKey} onChange={(e) => setNewKey(e.target.value)} placeholder="KEY" className="h-8 w-40 font-mono text-sm" autoFocus onKeyDown={(e) => { if (e.key === 'Enter') handleAdd(); if (e.key === 'Escape') setAdding(false); }} />
              <Input value={newValue} onChange={(e) => setNewValue(e.target.value)} placeholder="value" className="h-8 flex-1 font-mono text-sm" onKeyDown={(e) => { if (e.key === 'Enter') handleAdd(); if (e.key === 'Escape') setAdding(false); }} />
              <Button size="sm" onClick={handleAdd} disabled={!newKey.trim() || !newValue.trim()}><Check className="mr-1 size-3" />{t('common.save')}</Button>
              <Button size="sm" variant="ghost" onClick={() => setAdding(false)}><X className="mr-1 size-3" />{t('common.cancel')}</Button>
            </div>
          )}
        </div>
        {!adding && (
          <Button size="sm" variant="outline" className="mt-3" onClick={() => setAdding(true)}>
            <Plus className="mr-1 size-4" />
            {t('settingsEditor.addEnvVar')}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}
