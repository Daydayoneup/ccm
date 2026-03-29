import { cn } from '@/lib/utils';
import type { SettingsScope } from '@/lib/settings-utils';
import { useI18n } from '@/i18n/provider';

interface ScopeToggleProps {
  scope: SettingsScope;
  onScopeChange: (scope: SettingsScope) => void;
  filePath: string;
}

export function ScopeToggle({ scope, onScopeChange, filePath }: ScopeToggleProps) {
  const { t } = useI18n();
  const scopes: { value: SettingsScope; label: string }[] = [
    { value: 'shared', label: t('settingsEditor.scopeShared') },
    { value: 'local', label: t('settingsEditor.scopeLocal') },
  ];

  return (
    <div className="flex items-center gap-3">
      <div className="inline-flex rounded-lg border bg-muted/30 p-0.5">
        {scopes.map((s) => (
          <button
            key={s.value}
            onClick={() => onScopeChange(s.value)}
            className={cn(
              'rounded-md px-3 py-1.5 text-xs font-medium transition-all',
              scope === s.value
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            {s.label}
          </button>
        ))}
      </div>
      <code className="text-xs text-muted-foreground">{filePath}</code>
    </div>
  );
}
