import { create } from 'zustand';
import { listGlobalResources, createGlobalResource, deleteGlobalResource, backupToLibrary, syncScope } from '@/lib/tauri-api';
import { asyncAction } from '@/lib/store-utils';
import type { Resource, ResourceType } from '@/types/v2';

interface GlobalStore {
  resources: Resource[];
  loading: boolean;
  error: string | null;
  activeTab: ResourceType | 'plugin';

  loadResources: (resourceType?: ResourceType) => Promise<void>;
  createResource: (resourceType: ResourceType, name: string, content: string) => Promise<Resource>;
  deleteResource: (id: string, deleteFromDisk?: boolean) => Promise<void>;
  backupToLibrary: (resourceId: string, replaceWithLink?: boolean) => Promise<Resource>;
  setActiveTab: (tab: ResourceType | 'plugin') => void;
  getFilteredResources: () => Resource[];
}

export const useGlobalStore = create<GlobalStore>((set, get) => ({
  resources: [],
  loading: false,
  error: null,
  activeTab: 'skill',

  loadResources: async (resourceType?: ResourceType) => {
    const resources = await asyncAction(set, 'loading', () => listGlobalResources(resourceType));
    if (resources) set({ resources });
  },

  createResource: async (resourceType: ResourceType, name: string, content: string) => {
    const resource = await createGlobalResource(resourceType, name, content);
    await get().loadResources();
    return resource;
  },

  deleteResource: async (id: string, deleteFromDisk = false) => {
    set({ error: null });
    try {
      await deleteGlobalResource(id, deleteFromDisk);
      await get().loadResources();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  backupToLibrary: async (resourceId: string, replaceWithLink = false) => {
    const resource = await backupToLibrary(resourceId, replaceWithLink);
    if (replaceWithLink) {
      await syncScope('global');
    }
    await get().loadResources();
    return resource;
  },

  setActiveTab: (tab: ResourceType | 'plugin') => {
    set({ activeTab: tab });
  },

  getFilteredResources: () => {
    const { resources, activeTab } = get();
    return resources.filter((r) => r.resource_type === activeTab);
  },
}));
