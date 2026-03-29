import { create } from 'zustand';
import {
  listLibraryResourcesWithInstalls, createLibraryResource, deleteLibraryResource,
  installToProject, deployToGlobal, listResourceLinks, checkLinkHealth,
} from '@/lib/tauri-api';
import { asyncAction } from '@/lib/store-utils';
import type { LibraryResourceWithInstalls, Resource, ResourceLink, ResourceType, LinkType } from '@/types/v2';

interface LinkHealthInfo {
  link_id: string;
  target_path: string;
  healthy: boolean;
  error: string | null;
}

interface LibraryStore {
  resources: LibraryResourceWithInstalls[];
  links: ResourceLink[];
  linkHealth: LinkHealthInfo[];
  loading: boolean;
  error: string | null;
  activeTab: ResourceType;
  installFilter: 'all' | 'installed';

  loadResources: (resourceType?: ResourceType) => Promise<void>;
  createResource: (resourceType: ResourceType, name: string, description: string, content: string) => Promise<Resource>;
  deleteResource: (id: string, deleteFromDisk?: boolean) => Promise<void>;
  installToProject: (resourceId: string, projectId: string, linkType: LinkType) => Promise<void>;
  deployToGlobal: (resourceId: string, linkType: LinkType) => Promise<void>;
  loadLinks: (resourceId: string) => Promise<void>;
  checkLinkHealth: () => Promise<void>;
  setActiveTab: (tab: ResourceType) => void;
  setInstallFilter: (filter: 'all' | 'installed') => void;
  selectedSource: 'local' | string;
  setSelectedSource: (source: 'local' | string) => void;
  getFilteredResources: () => LibraryResourceWithInstalls[];
}

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  resources: [],
  links: [],
  linkHealth: [],
  loading: false,
  error: null,
  activeTab: 'skill',
  selectedSource: 'local',
  installFilter: 'all',

  loadResources: async (resourceType?: ResourceType) => {
    const resources = await asyncAction(set, 'loading', () => listLibraryResourcesWithInstalls(resourceType));
    if (resources) set({ resources });
  },

  createResource: async (resourceType: ResourceType, name: string, description: string, content: string) => {
    set({ loading: true, error: null });
    try {
      const resource = await createLibraryResource(resourceType, name, description, content);
      await get().loadResources();
      return resource;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  deleteResource: async (id: string, deleteFromDisk = false) => {
    await asyncAction(set, 'loading', async () => {
      await deleteLibraryResource(id, deleteFromDisk);
      await get().loadResources();
    });
  },

  installToProject: async (resourceId: string, projectId: string, linkType: LinkType) => {
    await asyncAction(set, 'loading', () => installToProject(resourceId, projectId, linkType));
  },

  deployToGlobal: async (resourceId: string, linkType: LinkType) => {
    await asyncAction(set, 'loading', () => deployToGlobal(resourceId, linkType));
  },

  loadLinks: async (resourceId: string) => {
    const links = await asyncAction(set, 'loading', () => listResourceLinks(resourceId));
    if (links) set({ links });
  },

  checkLinkHealth: async () => {
    const linkHealth = await asyncAction(set, 'loading', checkLinkHealth);
    if (linkHealth) set({ linkHealth });
  },

  setActiveTab: (tab: ResourceType) => {
    set({ activeTab: tab });
  },

  setInstallFilter: (filter: 'all' | 'installed') => {
    set({ installFilter: filter });
  },

  setSelectedSource: (source: 'local' | string) => {
    set({ selectedSource: source });
  },

  getFilteredResources: () => {
    const { resources, activeTab, installFilter } = get();
    let filtered = resources.filter((r) => r.resource.resource_type === activeTab);
    if (installFilter === 'installed') {
      filtered = filtered.filter((r) => r.installations.length > 0);
    }
    return filtered;
  },
}));
