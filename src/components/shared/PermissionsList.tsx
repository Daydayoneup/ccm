import { useEffect, useState } from 'react';
import { getProjectPermissions, updateProjectPermissions } from '@/lib/tauri-api';
import { Check, Pencil, Plus, Shield, ShieldOff, Trash2, X } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useI18n } from '@/i18n/provider';

interface Permissions {
  allow: string[];
  deny: string[];
}

interface PermissionsListProps {
  projectId: string;
}

function RuleItem({
  rule,
  onEdit,
  onRemove,
  editTitle,
  removeTitle,
  clickToEdit,
  saveLabel,
  cancelLabel,
}: {
  rule: string;
  onEdit: (newValue: string) => void;
  onRemove: () => void;
  editTitle: string;
  removeTitle: string;
  clickToEdit: string;
  saveLabel: string;
  cancelLabel: string;
}) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(rule);

  const handleSave = () => {
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== rule) onEdit(trimmed);
    setEditing(false);
  };

  const handleCancel = () => {
    setEditValue(rule);
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
            if (e.key === 'Escape') handleCancel();
          }}
          onBlur={handleSave}
        />
        <Button variant="ghost" size="icon" className="size-7 shrink-0" onClick={handleSave} title={saveLabel}>
          <Check className="size-3.5" />
        </Button>
        <Button variant="ghost" size="icon" className="size-7 shrink-0" onMouseDown={(e) => e.preventDefault()} onClick={handleCancel} title={cancelLabel}>
          <X className="size-3.5" />
        </Button>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-4 py-2">
      <code className="cursor-pointer text-sm hover:text-primary" onClick={() => setEditing(true)} title={clickToEdit}>
        {rule}
      </code>
      <div className="flex items-center gap-0.5">
        <Button variant="ghost" size="icon" className="size-7" onClick={() => setEditing(true)} title={editTitle}>
          <Pencil className="size-3.5" />
        </Button>
        <Button variant="ghost" size="icon" className="size-7" onClick={onRemove} title={removeTitle}>
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
  emptyLabel,
}: {
  title: string;
  icon: React.ElementType;
  rules: string[];
  variant: 'default' | 'destructive';
  onAdd: (rule: string) => void;
  onEdit: (index: number, newValue: string) => void;
  onRemove: (index: number) => void;
  placeholder: string;
  emptyLabel: string;
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
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <Icon className="size-4" />
        <h3 className="font-medium">{title}</h3>
        <Badge variant="outline" className="text-xs">{rules.length}</Badge>
      </div>
      <div className="rounded-[20px] border">
        {rules.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-muted-foreground">{emptyLabel}</div>
        ) : (
          <div className="divide-y">
            {rules.map((rule, index) => (
              <RuleItem
                key={`${rule}-${index}`}
                rule={rule}
                onEdit={(newValue) => onEdit(index, newValue)}
                onRemove={() => onRemove(index)}
                editTitle={t('permissions.editRule')}
                removeTitle={t('permissions.removeRule')}
                clickToEdit={t('permissions.clickToEdit')}
                saveLabel={t('common.save')}
                cancelLabel={t('common.cancel')}
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
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleAdd();
            }}
          />
          <Button size="sm" variant={variant} onClick={handleAdd} disabled={!newRule.trim()}>
            <Plus className="mr-1 size-3" />
            {t('permissions.add')}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function PermissionsList({ projectId }: PermissionsListProps) {
  const { t } = useI18n();
  const [permissions, setPermissions] = useState<Permissions>({ allow: [], deny: [] });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    getProjectPermissions(projectId)
      .then(setPermissions)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [projectId]);

  const save = async (updated: Permissions) => {
    setSaving(true);
    setError(null);
    try {
      await updateProjectPermissions(projectId, updated.allow, updated.deny);
      setPermissions(updated);
    } catch (error) {
      setError(String(error));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12 text-muted-foreground">
        {t('permissions.loading')}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error ? <div className="rounded-lg border border-destructive bg-destructive/10 p-3 text-sm text-destructive">{error}</div> : null}
      {saving ? <div className="text-xs text-muted-foreground">{t('permissions.saving')}</div> : null}
      <RuleSection
        title={t('permissions.allowRules')}
        icon={Shield}
        rules={permissions.allow}
        variant="default"
        onAdd={(rule) => save({ ...permissions, allow: [...permissions.allow, rule] })}
        onEdit={(index, newValue) => save({ ...permissions, allow: permissions.allow.map((rule, i) => i === index ? newValue : rule) })}
        onRemove={(index) => save({ ...permissions, allow: permissions.allow.filter((_, i) => i !== index) })}
        placeholder="e.g. Bash(npm run:*)"
        emptyLabel={t('permissions.noRules', { title: t('permissions.allowRules') })}
      />
      <RuleSection
        title={t('permissions.denyRules')}
        icon={ShieldOff}
        rules={permissions.deny}
        variant="destructive"
        onAdd={(rule) => save({ ...permissions, deny: [...permissions.deny, rule] })}
        onEdit={(index, newValue) => save({ ...permissions, deny: permissions.deny.map((rule, i) => i === index ? newValue : rule) })}
        onRemove={(index) => save({ ...permissions, deny: permissions.deny.filter((_, i) => i !== index) })}
        placeholder="e.g. Bash(rm -rf:*)"
        emptyLabel={t('permissions.noRules', { title: t('permissions.denyRules') })}
      />
    </div>
  );
}
