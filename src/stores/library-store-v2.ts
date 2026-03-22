import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Resource, ResourceLink, ResourceType, LinkType } from '@/types/v2';

interface LinkHealthInfo {
  link_id: string;
  target_path: string;
  healthy: boolean;
  error: string | null;
}

interface LibraryStore {
  resources: Resource[];
  links: ResourceLink[];
  linkHealth: LinkHealthInfo[];
  loading: boolean;
  error: string | null;
  activeTab: ResourceType;

  loadResources: (resourceType?: ResourceType) => Promise<void>;
  createResource: (resourceType: ResourceType, name: string, description: string, content: string) => Promise<Resource>;
  deleteResource: (id: string, deleteFromDisk?: boolean) => Promise<void>;
  installToProject: (resourceId: string, projectId: string, linkType: LinkType) => Promise<void>;
  deployToGlobal: (resourceId: string, linkType: LinkType) => Promise<void>;
  loadLinks: (resourceId: string) => Promise<void>;
  checkLinkHealth: () => Promise<void>;
  setActiveTab: (tab: ResourceType) => void;
  selectedSource: 'local' | string;
  setSelectedSource: (source: 'local' | string) => void;
  getFilteredResources: () => Resource[];
}

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  resources: [],
  links: [],
  linkHealth: [],
  loading: false,
  error: null,
  activeTab: 'skill',
  selectedSource: 'local',

  loadResources: async (resourceType?: ResourceType) => {
    set({ loading: true, error: null });
    try {
      const resources = await invoke<Resource[]>('list_library_resources', {
        resourceType: resourceType ?? null,
      });
      set({ resources, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createResource: async (resourceType: ResourceType, name: string, description: string, content: string) => {
    set({ loading: true, error: null });
    try {
      const resource = await invoke<Resource>('create_library_resource', {
        resourceType,
        name,
        description,
        content,
      });
      await get().loadResources();
      return resource;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  deleteResource: async (id: string, deleteFromDisk = false) => {
    set({ loading: true, error: null });
    try {
      await invoke<void>('delete_library_resource', { id, deleteFromDisk });
      await get().loadResources();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  installToProject: async (resourceId: string, projectId: string, linkType: LinkType) => {
    set({ loading: true, error: null });
    try {
      await invoke<void>('install_to_project', { resourceId, projectId, linkType });
      set({ loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  deployToGlobal: async (resourceId: string, linkType: LinkType) => {
    set({ loading: true, error: null });
    try {
      await invoke<void>('deploy_to_global', { resourceId, linkType });
      set({ loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  loadLinks: async (resourceId: string) => {
    set({ loading: true, error: null });
    try {
      const links = await invoke<ResourceLink[]>('list_resource_links', { resourceId });
      set({ links, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  checkLinkHealth: async () => {
    set({ loading: true, error: null });
    try {
      const linkHealth = await invoke<LinkHealthInfo[]>('check_link_health', {});
      set({ linkHealth, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setActiveTab: (tab: ResourceType) => {
    set({ activeTab: tab });
  },

  setSelectedSource: (source: 'local' | string) => {
    set({ selectedSource: source });
  },

  getFilteredResources: () => {
    const { resources, activeTab } = get();
    return resources.filter((r) => r.resource_type === activeTab);
  },
}));
