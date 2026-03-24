import { useState } from 'react';
import { Check, Eye, EyeOff, Plus, Trash2, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScopeBadge } from '@/lib/scope-utils';
import { useI18n } from '@/i18n/provider';
import type { MergedEnvVar } from '@/types/v2';

interface EnvVarTableProps {
  vars: MergedEnvVar[];
  onAdd: (key: string, value: string) => void;
  onDelete: (id: string) => void;
  readonlyScope?: string;
  onReadonlyClick?: () => void;
}

export function EnvVarTable({ vars, onAdd, onDelete, readonlyScope, onReadonlyClick }: EnvVarTableProps) {
  const { t } = useI18n();
  const [adding, setAdding] = useState(false);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [visibleIds, setVisibleIds] = useState<Set<string>>(new Set());

  const toggleVisibility = (id: string) => {
    setVisibleIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleAdd = () => {
    const trimmedKey = newKey.trim();
    const trimmedValue = newValue.trim();
    if (!trimmedKey || !trimmedValue) return;
    onAdd(trimmedKey, trimmedValue);
    setNewKey('');
    setNewValue('');
    setAdding(false);
  };

  const handleCancel = () => {
    setNewKey('');
    setNewValue('');
    setAdding(false);
  };

  return (
    <div className="space-y-3">
      <div className="rounded-[20px] border">
        {vars.length === 0 && !adding ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            {t('env.empty')}
          </div>
        ) : (
          <div className="divide-y">
            {vars.map((variable) => {
              const isReadonly = readonlyScope && variable.scope === readonlyScope;
              return (
                <div
                  key={variable.id}
                  className={`flex items-center gap-3 px-4 py-2.5${isReadonly ? ' cursor-pointer opacity-70 hover:opacity-100' : ''}`}
                  onClick={isReadonly ? onReadonlyClick : undefined}
                >
                  <code className="min-w-[140px] text-sm font-semibold">{variable.key}</code>
                  <div className="flex flex-1 items-center gap-2">
                    <code className="text-sm text-muted-foreground">
                      {visibleIds.has(variable.id) ? variable.value : '••••••••'}
                    </code>
                  </div>
                  {readonlyScope ? <ScopeBadge scope={variable.scope as 'global' | 'project'} className="shrink-0" /> : null}
                  <div className="flex items-center gap-0.5">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="size-7"
                      onClick={(e) => { e.stopPropagation(); toggleVisibility(variable.id); }}
                      title={visibleIds.has(variable.id) ? t('env.hideValue') : t('env.showValue')}
                    >
                      {visibleIds.has(variable.id) ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
                    </Button>
                    {!isReadonly ? (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-7"
                        onClick={() => onDelete(variable.id)}
                        title={t('common.delete')}
                      >
                        <Trash2 className="size-3.5" />
                      </Button>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        )}
        {adding ? (
          <div className="flex items-center gap-2 border-t px-3 py-2">
            <Input
              value={newKey}
              onChange={(e) => setNewKey(e.target.value)}
              placeholder="KEY"
              className="h-8 w-40 font-mono text-sm"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAdd();
                if (e.key === 'Escape') handleCancel();
              }}
            />
            <Input
              value={newValue}
              onChange={(e) => setNewValue(e.target.value)}
              placeholder="value"
              className="h-8 flex-1 font-mono text-sm"
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAdd();
                if (e.key === 'Escape') handleCancel();
              }}
            />
            <Button size="sm" onClick={handleAdd} disabled={!newKey.trim() || !newValue.trim()}>
              <Check className="mr-1 size-3" />
              {t('common.save')}
            </Button>
            <Button size="sm" variant="ghost" onClick={handleCancel}>
              <X className="mr-1 size-3" />
              {t('common.cancel')}
            </Button>
          </div>
        ) : null}
      </div>
      {!adding ? (
        <Button size="sm" variant="outline" onClick={() => setAdding(true)}>
          <Plus className="mr-1 size-4" />
          {t('env.addVariable')}
        </Button>
      ) : null}
    </div>
  );
}
