import { useState } from 'react';
import { Check, Pencil, Plus, Shield, ShieldAlert, ShieldOff, Trash2, X } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { useI18n } from '@/i18n/provider';
import type { SettingsPermissions } from '@/lib/settings-utils';

interface PermissionsCardProps {
  permissions: SettingsPermissions;
  onChange: (permissions: SettingsPermissions) => void;
}

function RuleItem({
  rule,
  onEdit,
  onRemove,
}: {
  rule: string;
  onEdit: (newValue: string) => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(rule);

  const handleSave = () => {
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== rule) onEdit(trimmed);
    setEditing(false);
  };

  if (editing) {
    return (
      <div className="flex items-center gap-1 px-4 py-1.5">
        <Input
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          className="h-7 text-sm font-mono"
          autoFocus
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleSave();
            if (e.key === 'Escape') { setEditValue(rule); setEditing(false); }
          }}
          onBlur={handleSave}
        />
        <Button variant="ghost" size="icon" className="size-7 shrink-0" onClick={handleSave}>
          <Check className="size-3.5" />
        </Button>
        <Button variant="ghost" size="icon" className="size-7 shrink-0" onMouseDown={(e) => e.preventDefault()} onClick={() => { setEditValue(rule); setEditing(false); }}>
          <X className="size-3.5" />
        </Button>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-4 py-2">
      <code className="cursor-pointer text-sm hover:text-primary" onClick={() => setEditing(true)} title={t('settingsEditor.clickToEdit')}>
        {rule}
      </code>
      <div className="flex items-center gap-0.5">
        <Button variant="ghost" size="icon" className="size-7" onClick={() => setEditing(true)}>
          <Pencil className="size-3.5" />
        </Button>
        <Button variant="ghost" size="icon" className="size-7" onClick={onRemove}>
          <Trash2 className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}

function RuleSection({
  title,
  icon: Icon,
  rules,
  variant,
  onAdd,
  onEdit,
  onRemove,
  placeholder,
}: {
  title: string;
  icon: React.ElementType;
  rules: string[];
  variant: 'default' | 'secondary' | 'destructive';
  onAdd: (rule: string) => void;
  onEdit: (index: number, newValue: string) => void;
  onRemove: (index: number) => void;
  placeholder: string;
}) {
  const { t } = useI18n();
  const [newRule, setNewRule] = useState('');

  const handleAdd = () => {
    const trimmed = newRule.trim();
    if (!trimmed) return;
    onAdd(trimmed);
    setNewRule('');
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2">
        <Icon className="size-4" />
        <span className="text-sm font-medium">{title}</span>
        <Badge variant="outline" className="text-xs">{rules.length}</Badge>
      </div>
      <div className="rounded-[20px] border">
        {rules.length === 0 ? (
          <div className="px-4 py-4 text-center text-xs text-muted-foreground">
            {t('settingsEditor.noRules')}
          </div>
        ) : (
          <div className="divide-y">
            {rules.map((rule, index) => (
              <RuleItem
                key={`${rule}-${index}`}
                rule={rule}
                onEdit={(v) => onEdit(index, v)}
                onRemove={() => onRemove(index)}
              />
            ))}
          </div>
        )}
        <div className="flex gap-2 border-t px-3 py-2">
          <Input
            value={newRule}
            onChange={(e) => setNewRule(e.target.value)}
            placeholder={placeholder}
            className="h-8 text-sm"
            onKeyDown={(e) => { if (e.key === 'Enter') handleAdd(); }}
          />
          <Button size="sm" variant={variant} onClick={handleAdd} disabled={!newRule.trim()}>
            <Plus className="mr-1 size-3" />
            {t('settingsEditor.add')}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function PermissionsCard({ permissions, onChange }: PermissionsCardProps) {
  const { t } = useI18n();

  const update = (field: keyof SettingsPermissions, updater: (rules: string[]) => string[]) => {
    onChange({ ...permissions, [field]: updater(permissions[field]) });
  };

  return (
    <Card>
      <CardHeader className="pb-4">
        <CardTitle className="flex items-center gap-2 text-base">
          <Shield className="size-4" />
          {t('settingsEditor.permissions')}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-5">
        <RuleSection
          title={t('settingsEditor.allowRules')}
          icon={Shield}
          rules={permissions.allow}
          variant="default"
          onAdd={(rule) => update('allow', (r) => [...r, rule])}
          onEdit={(i, v) => update('allow', (r) => r.map((x, j) => j === i ? v : x))}
          onRemove={(i) => update('allow', (r) => r.filter((_, j) => j !== i))}
          placeholder="e.g. Bash(npm run *)"
        />
        <RuleSection
          title={t('settingsEditor.askRules')}
          icon={ShieldAlert}
          rules={permissions.ask}
          variant="secondary"
          onAdd={(rule) => update('ask', (r) => [...r, rule])}
          onEdit={(i, v) => update('ask', (r) => r.map((x, j) => j === i ? v : x))}
          onRemove={(i) => update('ask', (r) => r.filter((_, j) => j !== i))}
          placeholder="e.g. Bash(git push *)"
        />
        <RuleSection
          title={t('settingsEditor.denyRules')}
          icon={ShieldOff}
          rules={permissions.deny}
          variant="destructive"
          onAdd={(rule) => update('deny', (r) => [...r, rule])}
          onEdit={(i, v) => update('deny', (r) => r.map((x, j) => j === i ? v : x))}
          onRemove={(i) => update('deny', (r) => r.filter((_, j) => j !== i))}
          placeholder="e.g. Bash(rm -rf *)"
        />
      </CardContent>
    </Card>
  );
}
