import { useNavigate } from 'react-router-dom';
import { Globe, FolderGit2, Package, Library, CloudCog } from 'lucide-react';
import type { DashboardStats } from '@/types/v2';

interface StatsCardsProps {
  stats: DashboardStats | null;
}

const cards = [
  {
    key: 'global',
    label: 'Global Resources',
    icon: Globe,
    countKey: 'global_count' as const,
    path: '/global',
    accent: 'from-res-skill/20 to-transparent',
    borderColor: 'border-l-res-skill',
    iconColor: 'text-res-skill',
  },
  {
    key: 'projects',
    label: 'Projects',
    icon: FolderGit2,
    countKey: 'project_count' as const,
    path: '/projects',
    accent: 'from-res-agent/20 to-transparent',
    borderColor: 'border-l-res-agent',
    iconColor: 'text-res-agent',
  },
  {
    key: 'plugins',
    label: 'Plugins',
    icon: Package,
    countKey: 'plugin_count' as const,
    path: '/global?tab=plugin',
    accent: 'from-res-rule/20 to-transparent',
    borderColor: 'border-l-res-rule',
    iconColor: 'text-res-rule',
  },
  {
    key: 'library',
    label: 'Library',
    icon: Library,
    countKey: 'library_count' as const,
    path: '/library',
    accent: 'from-res-hook/20 to-transparent',
    borderColor: 'border-l-res-hook',
    iconColor: 'text-res-hook',
  },
  {
    key: 'registries',
    label: 'Registries',
    icon: CloudCog,
    countKey: 'registry_count' as const,
    path: '/registries',
    accent: 'from-res-mcp/20 to-transparent',
    borderColor: 'border-l-res-mcp',
    iconColor: 'text-res-mcp',
  },
];

export function StatsCards({ stats }: StatsCardsProps) {
  const navigate = useNavigate();

  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-5">
      {cards.map((card) => {
        const Icon = card.icon;
        const count = stats ? stats[card.countKey] : 0;

        return (
          <div
            key={card.key}
            className={`card-glow group cursor-pointer rounded-xl border border-l-[3px] ${card.borderColor} bg-card p-4 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/5`}
            onClick={() => navigate(card.path)}
          >
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                {card.label}
              </span>
              <div className={`rounded-lg bg-muted p-1.5 ${card.iconColor} transition-colors group-hover:bg-primary/10`}>
                <Icon className="size-4" />
              </div>
            </div>
            <div className="mt-3 text-3xl font-bold tracking-tight">{count}</div>
          </div>
        );
      })}
    </div>
  );
}
