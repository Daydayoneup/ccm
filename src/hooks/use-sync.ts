import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useSyncStore } from '@/stores/sync-store';
import type { SyncProgress } from '@/stores/sync-store';

interface SyncReport {
  inserted: number;
  updated: number;
  deleted: number;
}

export function useSync() {
  const {
    addChangedPaths,
    setSyncStatus,
    setSyncProgress,
    setSyncError,
    setLastSynced,
  } = useSyncStore();

  useEffect(() => {
    const unlisteners = [
      listen<string[]>('fs-change', (event) => {
        addChangedPaths(event.payload);
      }),
      listen<SyncProgress>('sync-progress', (event) => {
        setSyncStatus('running');
        setSyncProgress(event.payload);
        setSyncError(null);
      }),
      listen<SyncReport>('sync-complete', (_event) => {
        setSyncStatus('idle');
        setSyncProgress(null);
        setLastSynced(new Date().toLocaleString());
      }),
      listen<string>('sync-error', (event) => {
        setSyncStatus('idle');
        setSyncProgress(null);
        setSyncError(event.payload);
      }),
    ];

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, [addChangedPaths, setSyncStatus, setSyncProgress, setSyncError, setLastSynced]);
}
