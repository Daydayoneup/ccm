import { create } from 'zustand';

export interface SyncProgress {
  stage: string;
  current: number;
  total: number;
  message: string;
}

interface SyncStore {
  syncStatus: 'idle' | 'running' | 'queued';
  syncProgress: SyncProgress | null;
  syncError: string | null;
  lastSynced: string | null;
  changedPaths: string[];

  setSyncStatus: (status: 'idle' | 'running' | 'queued') => void;
  setSyncProgress: (progress: SyncProgress | null) => void;
  setSyncError: (error: string | null) => void;
  setLastSynced: (time: string) => void;
  addChangedPaths: (paths: string[]) => void;
  clearChangedPaths: () => void;
}

export const useSyncStore = create<SyncStore>((set, get) => ({
  syncStatus: 'idle',
  syncProgress: null,
  syncError: null,
  lastSynced: null,
  changedPaths: [],

  setSyncStatus: (syncStatus) => set({ syncStatus }),
  setSyncProgress: (syncProgress) => set({ syncProgress }),
  setSyncError: (syncError) => set({ syncError }),
  setLastSynced: (time) => set({ lastSynced: time }),
  addChangedPaths: (paths) =>
    set({ changedPaths: [...get().changedPaths, ...paths] }),
  clearChangedPaths: () => set({ changedPaths: [] }),
}));
