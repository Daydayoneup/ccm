import { RefreshCw } from 'lucide-react';
import { fullSync } from '@/lib/tauri-api';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useI18n } from '@/i18n/provider';
import { useSyncStore } from '@/stores/sync-store';

export function SyncCard() {
  const { t } = useI18n();
  const { syncStatus, syncProgress, syncError, lastSynced } = useSyncStore();

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <RefreshCw className="size-5" />
          <CardTitle>{t('settings.sync')}</CardTitle>
        </div>
        <CardDescription>{t('settings.syncDescription')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center gap-4">
          <Button
            onClick={async () => {
              try {
                await fullSync();
              } catch (error) {
                console.error('Failed to trigger sync:', error);
              }
            }}
            disabled={syncStatus === 'queued'}
            size="sm"
          >
            <RefreshCw className={`mr-2 size-4 ${syncStatus === 'running' ? 'animate-spin' : ''}`} />
            {syncStatus === 'running'
              ? t('common.syncing')
              : syncStatus === 'queued'
                ? t('settings.queued')
                : t('settings.runFullSync')}
          </Button>
          {lastSynced ? <span className="text-sm text-muted-foreground">{t('settings.lastSynced', { value: lastSynced })}</span> : null}
        </div>
        {syncStatus === 'running' && syncProgress ? (
          <div className="rounded-md border border-blue-500/30 bg-blue-500/10 p-3 text-sm text-blue-700 dark:text-blue-400">
            {syncProgress.message}
          </div>
        ) : null}
        {syncError ? (
          <div className="rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
            {t('settings.syncFailed', { message: syncError })}
          </div>
        ) : null}
        {syncStatus === 'idle' && !syncError && lastSynced ? (
          <div className="rounded-md border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-700 dark:text-green-400">
            {t('settings.syncSuccess')}
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
