import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Plugin, Resource, ResourceType } from '@/types/v2';

interface PluginStore {
  plugins: Plugin[];
  selectedPlugin: Plugin | null;
  pluginResources: Resource[];
  loading: boolean;
  scanning: boolean;
  error: string | null;
  activeTab: ResourceType;

  loadPlugins: () => Promise<void>;
  scanPlugins: () => Promise<void>;
  selectPlugin: (plugin: Plugin | null) => void;
  loadPluginResources: (pluginId: string, resourceType?: ResourceType) => Promise<void>;
  extractToLibrary: (resourceId: string) => Promise<Resource>;
  setActiveTab: (tab: ResourceType) => void;
  getFilteredResources: () => Resource[];
}

export const usePluginStore = create<PluginStore>((set, get) => ({
  plugins: [],
  selectedPlugin: null,
  pluginResources: [],
  loading: false,
  scanning: false,
  error: null,
  activeTab: 'skill',

  loadPlugins: async () => {
    set({ loading: true, error: null });
    try {
      const plugins = await invoke<Plugin[]>('list_plugins_v2');
      set({ plugins, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  scanPlugins: async () => {
    set({ scanning: true, error: null });
    try {
      const plugins = await invoke<Plugin[]>('scan_plugins');
      set({ plugins, scanning: false });
    } catch (e) {
      set({ error: String(e), scanning: false });
    }
  },

  selectPlugin: (plugin: Plugin | null) => {
    set({ selectedPlugin: plugin, pluginResources: [] });
  },

  loadPluginResources: async (pluginId: string, resourceType?: ResourceType) => {
    set({ loading: true, error: null });
    try {
      const resources = await invoke<Resource[]>('get_plugin_resources', {
        pluginId,
        resourceType: resourceType ?? null,
      });
      set({ pluginResources: resources, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  extractToLibrary: async (resourceId: string) => {
    const resource = await invoke<Resource>('extract_to_library', { resourceId });
    return resource;
  },

  setActiveTab: (tab: ResourceType) => {
    set({ activeTab: tab });
  },

  getFilteredResources: () => {
    const { pluginResources, activeTab } = get();
    return pluginResources.filter((r) => r.resource_type === activeTab);
  },
}));
