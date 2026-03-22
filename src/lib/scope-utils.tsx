import { Globe, FolderOpen, BookMarked, Puzzle, GitBranch } from 'lucide-react';
import { Badge } from '@/components/ui/badge';

const scopeConfig: Record<string, {
  icon: React.ElementType;
  variant: 'default' | 'secondary' | 'outline';
  label: string;
  className: string;
}> = {
  global: { icon: Globe, variant: 'default', label: 'Global', className: 'bg-primary/15 text-primary border-primary/30 hover:bg-primary/20' },
  project: { icon: FolderOpen, variant: 'outline', label: 'Project', className: 'bg-res-agent/10 text-res-agent border-res-agent/30 hover:bg-res-agent/20' },
  library: { icon: BookMarked, variant: 'secondary', label: 'Library', className: 'bg-res-rule/10 text-res-rule border-res-rule/30 hover:bg-res-rule/20' },
  plugin: { icon: Puzzle, variant: 'secondary', label: 'Plugin', className: 'bg-res-hook/10 text-res-hook border-res-hook/30 hover:bg-res-hook/20' },
  registry: { icon: GitBranch, variant: 'secondary', label: 'Registry', className: 'bg-res-mcp/10 text-res-mcp border-res-mcp/30 hover:bg-res-mcp/20' },
};

interface ScopeBadgeProps {
  scope: string;
  className?: string;
}

export function ScopeBadge({ scope, className }: ScopeBadgeProps) {
  const config = scopeConfig[scope] ?? { icon: Globe, variant: 'outline' as const, label: scope, className: '' };
  const Icon = config.icon;

  return (
    <Badge variant="outline" className={`${config.className} ${className ?? ''}`}>
      <Icon className="mr-1 size-3" />
      {config.label}
    </Badge>
  );
}
