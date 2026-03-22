import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Plus, Trash2, Check, X, Eye, EyeOff } from 'lucide-react';
import { ScopeBadge } from '@/lib/scope-utils';
import type { MergedEnvVar } from '@/types/v2';

interface EnvVarTableProps {
  vars: MergedEnvVar[];
  onAdd: (key: string, value: string) => void;
  onDelete: (id: string) => void;
  readonlyScope?: string;
  onReadonlyClick?: () => void;
}

export function EnvVarTable({ vars, onAdd, onDelete, readonlyScope, onReadonlyClick }: EnvVarTableProps) {
  const [adding, setAdding] = useState(false);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [visibleIds, setVisibleIds] = useState<Set<string>>(new Set());

  const toggleVisibility = (id: string) => {
    setVisibleIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
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
      <div className="rounded-lg border">
        {vars.length === 0 && !adding ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            No environment variables configured.
          </div>
        ) : (
          <div className="divide-y">
            {vars.map((v) => {
              const isReadonly = readonlyScope && v.scope === readonlyScope;
              return (
                <div
                  key={v.id}
                  className={`flex items-center gap-3 px-4 py-2.5${isReadonly ? ' cursor-pointer opacity-70 hover:opacity-100' : ''}`}
                  onClick={isReadonly ? onReadonlyClick : undefined}
                >
                  <code className="min-w-[140px] text-sm font-semibold">{v.key}</code>
                  <div className="flex flex-1 items-center gap-2">
                    <code className="text-sm text-muted-foreground">
                      {visibleIds.has(v.id) ? v.value : '••••••••'}
                    </code>
                  </div>
                  {readonlyScope && (
                    <ScopeBadge scope={v.scope as 'global' | 'project'} className="shrink-0" />
                  )}
                  <div className="flex items-center gap-0.5">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="size-7"
                      onClick={(e) => { e.stopPropagation(); toggleVisibility(v.id); }}
                      title={visibleIds.has(v.id) ? 'Hide value' : 'Show value'}
                    >
                      {visibleIds.has(v.id) ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
                    </Button>
                    {!isReadonly && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-7"
                        onClick={() => onDelete(v.id)}
                        title="Delete"
                      >
                        <Trash2 className="size-3.5" />
                      </Button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
        {adding && (
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
              Save
            </Button>
            <Button size="sm" variant="ghost" onClick={handleCancel}>
              <X className="mr-1 size-3" />
              Cancel
            </Button>
          </div>
        )}
      </div>
      {!adding && (
        <Button size="sm" variant="outline" onClick={() => setAdding(true)}>
          <Plus className="mr-1 size-4" />
          Add Variable
        </Button>
      )}
    </div>
  );
}
