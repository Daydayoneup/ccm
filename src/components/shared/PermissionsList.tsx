import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Plus, Trash2, Shield, ShieldOff, Pencil, Check, X } from 'lucide-react';

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
}: {
  rule: string;
  onEdit: (newValue: string) => void;
  onRemove: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(rule);

  const handleSave = () => {
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== rule) {
      onEdit(trimmed);
    }
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
        <Button variant="ghost" size="icon" className="size-7 shrink-0" onClick={handleSave} title="Save">
          <Check className="size-3.5" />
        </Button>
        <Button variant="ghost" size="icon" className="size-7 shrink-0" onMouseDown={(e) => e.preventDefault()} onClick={handleCancel} title="Cancel">
          <X className="size-3.5" />
        </Button>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-4 py-2">
      <code className="text-sm cursor-pointer hover:text-primary" onClick={() => setEditing(true)} title="Click to edit">
        {rule}
      </code>
      <div className="flex items-center gap-0.5">
        <Button variant="ghost" size="icon" className="size-7" onClick={() => setEditing(true)} title="Edit rule">
          <Pencil className="size-3.5" />
        </Button>
        <Button variant="ghost" size="icon" className="size-7" onClick={onRemove} title="Remove rule">
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
  variant: 'default' | 'destructive';
  onAdd: (rule: string) => void;
  onEdit: (index: number, newValue: string) => void;
  onRemove: (index: number) => void;
  placeholder: string;
}) {
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
      <div className="rounded-lg border">
        {rules.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-muted-foreground">
            No {title.toLowerCase()} configured.
          </div>
        ) : (
          <div className="divide-y">
            {rules.map((rule, i) => (
              <RuleItem
                key={`${rule}-${i}`}
                rule={rule}
                onEdit={(newValue) => onEdit(i, newValue)}
                onRemove={() => onRemove(i)}
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
            Add
          </Button>
        </div>
      </div>
    </div>
  );
}

export function PermissionsList({ projectId }: PermissionsListProps) {
  const [permissions, setPermissions] = useState<Permissions>({ allow: [], deny: [] });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    invoke<Permissions>('get_project_permissions', { projectId })
      .then(setPermissions)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [projectId]);

  const save = async (updated: Permissions) => {
    setSaving(true);
    setError(null);
    try {
      await invoke('update_project_permissions', {
        projectId,
        allow: updated.allow,
        deny: updated.deny,
      });
      setPermissions(updated);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const addAllow = (rule: string) => save({ ...permissions, allow: [...permissions.allow, rule] });
  const editAllow = (index: number, newValue: string) => save({ ...permissions, allow: permissions.allow.map((r, i) => i === index ? newValue : r) });
  const removeAllow = (index: number) => save({ ...permissions, allow: permissions.allow.filter((_, i) => i !== index) });

  const addDeny = (rule: string) => save({ ...permissions, deny: [...permissions.deny, rule] });
  const editDeny = (index: number, newValue: string) => save({ ...permissions, deny: permissions.deny.map((r, i) => i === index ? newValue : r) });
  const removeDeny = (index: number) => save({ ...permissions, deny: permissions.deny.filter((_, i) => i !== index) });

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12 text-muted-foreground">
        Loading permissions...
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error && (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}
      {saving && (
        <div className="text-xs text-muted-foreground">Saving...</div>
      )}
      <RuleSection
        title="Allow Rules"
        icon={Shield}
        rules={permissions.allow}
        variant="default"
        onAdd={addAllow}
        onEdit={editAllow}
        onRemove={removeAllow}
        placeholder='e.g. Bash(npm run:*)'
      />
      <RuleSection
        title="Deny Rules"
        icon={ShieldOff}
        rules={permissions.deny}
        variant="destructive"
        onAdd={addDeny}
        onEdit={editDeny}
        onRemove={removeDeny}
        placeholder='e.g. Bash(rm -rf:*)'
      />
    </div>
  );
}
