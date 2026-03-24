import { Badge } from '@/components/ui/badge';
import { ScopeBadge } from '@/lib/scope-utils';
import { useNavigate } from 'react-router-dom';
import type { Resource } from '@/types/v2';
import { useI18n } from '@/i18n/provider';

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

export function RecentChanges({ resources }: RecentChangesProps) {
  const { t, formatRelativeTime } = useI18n();
  const navigate = useNavigate();

  if (resources.length === 0) {
    return (
      <div className="rounded-md border border-dashed p-8 text-center text-sm text-muted-foreground">
        {t('dashboard.noRecent')}
      </div>
    );
  }

  return (
    <div className="rounded-md border border-border/60 bg-card/90">
      <div className="divide-y divide-border">
        {resources.map((resource) => (
          <div
            key={resource.id}
            className="group flex cursor-pointer items-center justify-between gap-4 px-4 py-3 transition-colors hover:bg-accent/30"
            onClick={() => {
              const filePath = resource.source_path;
              const extra = resource.resource_type === 'skill'
                ? `&resource_id=${resource.id}&type=skill&scope=${resource.scope === 'project' ? 'project' : 'library'}`
                : '';
              navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
            }}
          >
            <div className="flex min-w-0 items-center gap-3">
              <div className={`size-2 rounded-full ${typeColors[resource.resource_type]?.split(' ')[0] || 'bg-muted'}`} />
              <span className="truncate text-sm font-medium">{resource.name}</span>
              <Badge
                variant="outline"
                className={`shrink-0 text-[10px] font-medium ${typeColors[resource.resource_type] || ''}`}
              >
                {t(`resourceTypes.${resource.resource_type}`)}
              </Badge>
              <ScopeBadge scope={resource.scope} className="shrink-0" />
            </div>
            <span className="ml-4 whitespace-nowrap text-[11px] font-mono tabular-nums text-muted-foreground">
              {formatRelativeTime(resource.updated_at)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
