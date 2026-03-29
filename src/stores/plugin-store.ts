import { create } from 'zustand';
import { listPluginsV2, scanPlugins, getPluginResources, extractToLibrary } from '@/lib/tauri-api';
import { asyncAction } from '@/lib/store-utils';
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
    const plugins = await asyncAction(set, 'loading', listPluginsV2);
    if (plugins) set({ plugins });
  },

  scanPlugins: async () => {
    const plugins = await asyncAction(set, 'scanning', scanPlugins);
    if (plugins) set({ plugins });
  },

  selectPlugin: (plugin: Plugin | null) => {
    set({ selectedPlugin: plugin, pluginResources: [] });
  },

  loadPluginResources: async (pluginId: string, resourceType?: ResourceType) => {
    const resources = await asyncAction(set, 'loading', () => getPluginResources(pluginId, resourceType));
    if (resources) set({ pluginResources: resources });
  },

  extractToLibrary: async (resourceId: string) => {
    const resource = await extractToLibrary(resourceId);
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
