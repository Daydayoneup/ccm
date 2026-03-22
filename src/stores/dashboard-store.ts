import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { DashboardStats, Resource } from '@/types/v2';

interface DashboardStore {
  stats: DashboardStats | null;
  recentResources: Resource[];
  searchResults: Resource[];
  searchQuery: string;
  loading: boolean;
  error: string | null;

  loadStats: () => Promise<void>;
  loadRecent: (limit?: number) => Promise<void>;
  search: (query: string) => Promise<void>;
  clearSearch: () => void;
}

export const useDashboardStore = create<DashboardStore>((set) => ({
  stats: null,
  recentResources: [],
  searchResults: [],
  searchQuery: '',
  loading: false,
  error: null,

  loadStats: async () => {
    try {
      const stats = await invoke<DashboardStats>('get_dashboard_stats');
      set({ stats });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadRecent: async (limit = 10) => {
    try {
      const recentResources = await invoke<Resource[]>('get_recent_resources', { limit });
      set({ recentResources });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  search: async (query: string) => {
    set({ searchQuery: query });
    if (!query.trim()) {
      set({ searchResults: [] });
      return;
    }
    try {
      const searchResults = await invoke<Resource[]>('search_resources', { query });
      set({ searchResults });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearSearch: () => set({ searchQuery: '', searchResults: [] }),
}));
