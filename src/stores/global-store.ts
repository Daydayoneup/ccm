import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Resource, ResourceType } from '@/types/v2';
import type { McpServerInfo } from '@/components/shared/McpServerList';

interface GlobalStore {
  resources: Resource[];
  globalMcpServers: McpServerInfo[];
  loading: boolean;
  error: string | null;
  activeTab: ResourceType | 'mcp' | 'plugin';

  loadResources: (resourceType?: ResourceType) => Promise<void>;
  loadGlobalMcpServers: () => Promise<void>;
  createResource: (resourceType: ResourceType, name: string, content: string) => Promise<Resource>;
  deleteResource: (id: string, deleteFromDisk?: boolean) => Promise<void>;
  backupToLibrary: (resourceId: string) => Promise<Resource>;
  setActiveTab: (tab: ResourceType | 'mcp' | 'plugin') => void;
  getFilteredResources: () => Resource[];
}

export const useGlobalStore = create<GlobalStore>((set, get) => ({
  resources: [],
  globalMcpServers: [],
  loading: false,
  error: null,
  activeTab: 'skill',

  loadResources: async (resourceType?: ResourceType) => {
    set({ loading: true, error: null });
    try {
      const resources = await invoke<Resource[]>('list_global_resources', {
        resourceType: resourceType ?? null,
      });
      set({ resources, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  loadGlobalMcpServers: async () => {
    try {
      const servers = await invoke<McpServerInfo[]>('list_global_mcp_servers');
      set({ globalMcpServers: servers });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createResource: async (resourceType: ResourceType, name: string, content: string) => {
    const resource = await invoke<Resource>('create_global_resource', {
      resourceType,
      name,
      content,
    });
    // Reload resources after creation
    await get().loadResources();
    return resource;
  },

  deleteResource: async (id: string, deleteFromDisk = false) => {
    set({ error: null });
    try {
      await invoke<void>('delete_global_resource', { id, deleteFromDisk });
      await get().loadResources();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  backupToLibrary: async (resourceId: string) => {
    const resource = await invoke<Resource>('backup_to_library', { resourceId });
    return resource;
  },

  setActiveTab: (tab: ResourceType | 'mcp' | 'plugin') => {
    set({ activeTab: tab });
  },

  getFilteredResources: () => {
    const { resources, activeTab } = get();
    return resources.filter((r) => r.resource_type === activeTab);
  },
}));
