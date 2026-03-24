import { useNavigate } from 'react-router-dom';
import { Globe, FolderGit2, Package, Library, CloudCog } from 'lucide-react';
import type { DashboardStats } from '@/types/v2';
import { useI18n } from '@/i18n/provider';

interface StatsCardsProps {
  stats: DashboardStats | null;
}

const cards = [
  {
    key: 'global',
    labelKey: 'dashboard.metrics.global',
    icon: Globe,
    countKey: 'global_count' as const,
    path: '/global',
    borderColor: 'border-l-res-skill',
    iconColor: 'text-res-skill',
  },
  {
    key: 'projects',
    labelKey: 'dashboard.metrics.projects',
    icon: FolderGit2,
    countKey: 'project_count' as const,
    path: '/projects',
    borderColor: 'border-l-res-agent',
    iconColor: 'text-res-agent',
  },
  {
    key: 'plugins',
    labelKey: 'dashboard.metrics.plugins',
    icon: Package,
    countKey: 'plugin_count' as const,
    path: '/global?tab=plugin',
    borderColor: 'border-l-res-rule',
    iconColor: 'text-res-rule',
  },
  {
    key: 'library',
    labelKey: 'dashboard.metrics.library',
    icon: Library,
    countKey: 'library_count' as const,
    path: '/library',
    borderColor: 'border-l-res-hook',
    iconColor: 'text-res-hook',
  },
  {
    key: 'registries',
    labelKey: 'dashboard.metrics.registries',
    icon: CloudCog,
    countKey: 'registry_count' as const,
    path: '/registries',
    borderColor: 'border-l-res-mcp',
    iconColor: 'text-res-mcp',
  },
];

export function StatsCards({ stats }: StatsCardsProps) {
  const { t, formatNumber } = useI18n();
  const navigate = useNavigate();

  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-5">
      {cards.map((card) => {
        const Icon = card.icon;
        const count = stats ? stats[card.countKey] : 0;

        return (
          <div
            key={card.key}
            className={`card-glow group cursor-pointer rounded-md border border-l-[3px] ${card.borderColor} bg-card/90 p-5 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-[0_20px_50px_rgba(15,23,42,0.10)]`}
            onClick={() => navigate(card.path)}
          >
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
                {t(card.labelKey)}
              </span>
              <div className={`rounded-sm bg-panel p-2 ${card.iconColor} transition-colors group-hover:bg-primary/10`}>
                <Icon className="size-4" />
              </div>
            </div>
            <div className="mt-4 text-3xl font-semibold tracking-tight">{formatNumber(count)}</div>
          </div>
        );
      })}
    </div>
  );
}
