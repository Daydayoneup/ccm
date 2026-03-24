import { useEffect } from 'react';
import { GlobalSearch } from '@/components/dashboard/GlobalSearch';
import { RecentChanges } from '@/components/dashboard/RecentChanges';
import { StatsCards } from '@/components/dashboard/StatsCards';
import { PageHeader, PageShell, PanelSection } from '@/components/layout/PageShell';
import { useDashboardStore } from '@/stores/dashboard-store';
import { useI18n } from '@/i18n/provider';

export function DashboardPageV2() {
  const { t } = useI18n();
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
    <PageShell className="gap-5">
      <PageHeader
        eyebrow={t('nav.dashboard')}
        title={t('dashboard.title')}
        description={t('dashboard.subtitle')}
        actions={<GlobalSearch onSearch={search} results={searchResults} query={searchQuery} />}
      />
      <StatsCards stats={stats} />
      <PanelSection title={t('dashboard.recentChanges')}>
        <RecentChanges resources={recentResources} />
      </PanelSection>
    </PageShell>
  );
}
