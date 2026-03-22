import { Badge } from '@/components/ui/badge';
import { ScopeBadge } from '@/lib/scope-utils';
import { useNavigate } from 'react-router-dom';
import type { Resource } from '@/types/v2';

interface RecentChangesProps {
  resources: Resource[];
}

const typeColors: Record<string, string> = {
  skill: 'bg-res-skill/10 text-res-skill border-res-skill/30',
  agent: 'bg-res-agent/10 text-res-agent border-res-agent/30',
  rule: 'bg-res-rule/10 text-res-rule border-res-rule/30',
  hook: 'bg-res-hook/10 text-res-hook border-res-hook/30',
  mcp_server: 'bg-res-mcp/10 text-res-mcp border-res-mcp/30',
  command: 'bg-res-command/10 text-res-command border-res-command/30',
};

function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMinutes = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMinutes < 1) return 'just now';
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 30) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}


export function RecentChanges({ resources }: RecentChangesProps) {
  const navigate = useNavigate();

  if (resources.length === 0) {
    return (
      <div className="rounded-xl border border-dashed p-8 text-center text-sm text-muted-foreground">
        No resources found. Run a sync to discover resources.
      </div>
    );
  }

  return (
    <div className="rounded-xl border bg-card">
      <div className="divide-y divide-border">
        {resources.map((resource) => (
          <div
            key={resource.id}
            className="group flex items-center justify-between px-4 py-3 transition-colors hover:bg-accent/30 cursor-pointer"
            onClick={() =>
              navigate(`/editor?file=${encodeURIComponent(resource.source_path)}`)
            }
          >
            <div className="flex items-center gap-3 min-w-0">
              <div className={`size-2 rounded-full ${typeColors[resource.resource_type]?.split(' ')[0] || 'bg-muted'}`} />
              <span className="truncate font-medium text-sm">{resource.name}</span>
              <Badge
                variant="outline"
                className={`shrink-0 text-[10px] font-medium ${typeColors[resource.resource_type] || ''}`}
              >
                {resource.resource_type}
              </Badge>
              <ScopeBadge scope={resource.scope} className="shrink-0" />
            </div>
            <span className="text-[11px] font-mono text-muted-foreground whitespace-nowrap ml-4 tabular-nums">
              {formatRelativeTime(resource.updated_at)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
