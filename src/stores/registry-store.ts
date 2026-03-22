import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Registry, Resource, ResourceType, RegistryPlugin, McpServer } from '@/types/v2';
import { listRegistryPlugins, getRegistryPluginResources, getRegistryPluginMcpServers, installPluginToProject, installPluginToGlobal } from '@/lib/tauri-api';

interface RegistryStore {
  registries: Registry[];
  activeRegistry: Registry | null;
  resources: Resource[];
  loading: boolean;
  syncing: boolean;
  error: string | null;
  activeTab: ResourceType | 'mcp';

  loadRegistries: () => Promise<void>;
  addRegistry: (name: string, url: string, readonly: boolean) => Promise<Registry>;
  removeRegistry: (id: string) => Promise<void>;
  syncRegistry: (id: string) => Promise<void>;
  syncAll: () => Promise<void>;
  pushRegistry: (id: string, message: string) => Promise<void>;
  checkUpdates: () => Promise<void>;
  loadResources: (registryId: string, resourceType?: ResourceType) => Promise<void>;
  publishToRegistry: (resourceId: string, registryId: string) => Promise<Resource>;
  selectRegistry: (registry: Registry | null) => void;
  setActiveTab: (tab: ResourceType | 'mcp') => void;
  getFilteredResources: () => Resource[];

  registryPlugins: RegistryPlugin[];
  pluginResources: Record<string, Resource[]>;
  expandedPlugins: Set<string>;

  loadRegistryPlugins: (registryId: string) => Promise<void>;
  loadPluginResources: (pluginId: string) => Promise<void>;
  togglePluginExpanded: (pluginId: string) => void;
  pluginMcpServers: Record<string, McpServer[]>;
  loadPluginMcpServers: (pluginId: string) => Promise<void>;
  installPluginToProject: (pluginId: string, projectId: string) => Promise<void>;
  installPluginToGlobal: (pluginId: string) => Promise<void>;
}

export const useRegistryStore = create<RegistryStore>((set, get) => ({
  registries: [],
  activeRegistry: null,
  resources: [],
  loading: false,
  syncing: false,
  error: null,
  activeTab: 'skill',
  registryPlugins: [],
  pluginResources: {},
  expandedPlugins: new Set(),
  pluginMcpServers: {},

  loadRegistries: async () => {
    set({ loading: true, error: null });
    try {
      const registries = await invoke<Registry[]>('list_registries');
      set({ registries, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  addRegistry: async (name: string, url: string, readonly: boolean) => {
    set({ loading: true, error: null });
    try {
      const registry = await invoke<Registry>('add_registry', { name, url, readonly });
      const registries = await invoke<Registry[]>('list_registries');
      set({ registries, loading: false });
      return registry;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  removeRegistry: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await invoke('remove_registry', { id });
      const registries = await invoke<Registry[]>('list_registries');
      set({ registries, activeRegistry: null, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  syncRegistry: async (id: string) => {
    set({ syncing: true, error: null });
    try {
      await invoke('sync_registry', { id });
      const registries = await invoke<Registry[]>('list_registries');
      const activeRegistry = registries.find((r) => r.id === id) ?? null;
      set({ registries, activeRegistry, syncing: false });
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },

  syncAll: async () => {
    set({ syncing: true, error: null });
    try {
      const registries = await invoke<Registry[]>('sync_all_registries');
      set({ registries, syncing: false });
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },

  pushRegistry: async (id: string, message: string) => {
    set({ syncing: true, error: null });
    try {
      await invoke('push_registry', { id, message });
      const registries = await invoke<Registry[]>('list_registries');
      set({ registries, syncing: false });
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },

  checkUpdates: async () => {
    try {
      const registries = await invoke<Registry[]>('check_registry_updates');
      set({ registries });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadResources: async (registryId: string, resourceType?: ResourceType) => {
    set({ loading: true, error: null });
    try {
      const resources = await invoke<Resource[]>('list_registry_resources', {
        registryId,
        resourceType: resourceType ?? null,
      });
      set({ resources, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  publishToRegistry: async (resourceId: string, registryId: string) => {
    const resource = await invoke<Resource>('publish_to_registry', { resourceId, registryId });
    return resource;
  },

  selectRegistry: (registry: Registry | null) => {
    set({ activeRegistry: registry, resources: [] });
  },

  setActiveTab: (tab: ResourceType | 'mcp') => {
    set({ activeTab: tab });
  },

  getFilteredResources: () => {
    const { resources, activeTab } = get();
    if (activeTab === 'mcp') return [];
    return resources.filter((r) => r.resource_type === activeTab);
  },

  loadRegistryPlugins: async (registryId: string) => {
    try {
      const plugins = await listRegistryPlugins(registryId);
      set({ registryPlugins: plugins });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadPluginResources: async (pluginId: string) => {
    try {
      const resources = await getRegistryPluginResources(pluginId);
      set((state) => ({
        pluginResources: { ...state.pluginResources, [pluginId]: resources },
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadPluginMcpServers: async (pluginId: string) => {
    try {
      const servers = await getRegistryPluginMcpServers(pluginId);
      set((state) => ({
        pluginMcpServers: { ...state.pluginMcpServers, [pluginId]: servers },
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  togglePluginExpanded: (pluginId: string) => {
    set((state) => {
      const expanded = new Set(state.expandedPlugins);
      if (expanded.has(pluginId)) {
        expanded.delete(pluginId);
      } else {
        expanded.add(pluginId);
      }
      return { expandedPlugins: expanded };
    });
  },

  installPluginToProject: async (pluginId: string, projectId: string) => {
    try {
      set({ syncing: true });
      await installPluginToProject(pluginId, projectId);
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ syncing: false });
    }
  },

  installPluginToGlobal: async (pluginId: string) => {
    try {
      set({ syncing: true });
      await installPluginToGlobal(pluginId);
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ syncing: false });
    }
  },
}));
