import { Globe, FolderOpen, BookMarked, Puzzle, GitBranch } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { useI18n } from '@/i18n/provider';

const scopeConfig: Record<string, {
  icon: React.ElementType;
  className: string;
}> = {
  global: { icon: Globe, className: 'bg-primary/12 text-primary border-primary/25 hover:bg-primary/18' },
  project: { icon: FolderOpen, className: 'bg-res-agent/10 text-res-agent border-res-agent/30 hover:bg-res-agent/18' },
  library: { icon: BookMarked, className: 'bg-res-rule/10 text-res-rule border-res-rule/30 hover:bg-res-rule/18' },
  plugin: { icon: Puzzle, className: 'bg-res-hook/10 text-res-hook border-res-hook/30 hover:bg-res-hook/18' },
  registry: { icon: GitBranch, className: 'bg-res-mcp/10 text-res-mcp border-res-mcp/30 hover:bg-res-mcp/18' },
};

interface ScopeBadgeProps {
  scope: string;
  className?: string;
}

export function ScopeBadge({ scope, className }: ScopeBadgeProps) {
  const { t } = useI18n();
  const config = scopeConfig[scope] ?? { icon: Globe, className: '' };
  const Icon = config.icon;

  return (
    <Badge variant="outline" className={`${config.className} ${className ?? ''}`}>
      <Icon className="mr-1 size-3" />
      {t(`scopes.${scope}`)}
    </Badge>
  );
}
