import { create } from 'zustand';
import type { LibraryPlugin, Resource } from '@/types/v2';
import {
  listLibraryPlugins,
  createLibraryPlugin,
  deleteLibraryPlugin,
  addResourceToLibraryPlugin,
  removeResourceFromLibraryPlugin,
  getLibraryPluginResources,
} from '@/lib/tauri-api';
import { asyncAction } from '@/lib/store-utils';

interface LibraryPluginStore {
  plugins: LibraryPlugin[];
  activePlugin: LibraryPlugin | null;
  resources: Resource[];
  loading: boolean;
  error: string | null;

  loadPlugins: () => Promise<void>;
  createPlugin: (name: string, description: string | null, category: string | null) => Promise<void>;
  deletePlugin: (id: string) => Promise<void>;
  setActivePlugin: (plugin: LibraryPlugin | null) => void;
  loadPluginResources: (pluginId: string) => Promise<void>;
  addResource: (pluginId: string, resourceId: string) => Promise<void>;
  removeResource: (pluginId: string, resourceId: string) => Promise<void>;
}

export const useLibraryPluginStore = create<LibraryPluginStore>((set, get) => ({
  plugins: [],
  activePlugin: null,
  resources: [],
  loading: false,
  error: null,

  loadPlugins: async () => {
    const plugins = await asyncAction(set, 'loading', listLibraryPlugins);
    if (plugins) set({ plugins });
  },

  createPlugin: async (name, description, category) => {
    try {
      await createLibraryPlugin(name, description, category);
      await get().loadPlugins();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deletePlugin: async (id) => {
    try {
      await deleteLibraryPlugin(id);
      set({ activePlugin: null, resources: [] });
      await get().loadPlugins();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setActivePlugin: (plugin) => {
    set({ activePlugin: plugin });
    if (plugin) {
      get().loadPluginResources(plugin.id);
    }
  },

  loadPluginResources: async (pluginId) => {
    const resources = await asyncAction(set, 'loading', () => getLibraryPluginResources(pluginId));
    if (resources) set({ resources });
  },

  addResource: async (pluginId, resourceId) => {
    try {
      await addResourceToLibraryPlugin(pluginId, resourceId);
      await get().loadPluginResources(pluginId);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  removeResource: async (pluginId, resourceId) => {
    try {
      await removeResourceFromLibraryPlugin(pluginId, resourceId);
      await get().loadPluginResources(pluginId);
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
