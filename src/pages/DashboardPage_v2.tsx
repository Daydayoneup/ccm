import { useEffect } from 'react';
import { useDashboardStore } from '@/stores/dashboard-store';
import { StatsCards } from '@/components/dashboard/StatsCards';
import { RecentChanges } from '@/components/dashboard/RecentChanges';
import { GlobalSearch } from '@/components/dashboard/GlobalSearch';

export function DashboardPageV2() {
  const {
    stats,
    recentResources,
    searchResults,
    searchQuery,
    loadStats,
    loadRecent,
    search,
  } = useDashboardStore();

  useEffect(() => {
    loadStats();
    loadRecent();
  }, [loadStats, loadRecent]);

  return (
    <div className="space-y-8 p-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Overview of your Claude Code resources
          </p>
        </div>
        <GlobalSearch onSearch={search} results={searchResults} query={searchQuery} />
      </div>
      <StatsCards stats={stats} />
      <div>
        <h2 className="mb-4 text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Recent Changes
        </h2>
        <RecentChanges resources={recentResources} />
      </div>
    </div>
  );
}
