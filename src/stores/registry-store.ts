import { create } from 'zustand';
import type { Registry, Resource, ResourceType, RegistryPlugin, ResourceLink } from '@/types/v2';
import {
  listRegistries, addRegistry as apiAddRegistry, removeRegistry as apiRemoveRegistry,
  syncRegistry as apiSyncRegistry, syncAllRegistries, checkRegistryUpdates,
  listRegistryResources, publishToRegistry as apiPublishToRegistry, pushRegistry as apiPushRegistry,
  listRegistryPlugins, getRegistryPluginResources, installPluginToProject, installPluginToGlobal,
  installResourceToProject as apiInstallResourceToProject, installResourceToGlobal as apiInstallResourceToGlobal,
  uninstallResource as apiUninstallResource, getPluginResourcesInstallStatus,
  updateInstalledResource as apiUpdateInstalledResource, retainAsLibrary as apiRetainAsLibrary,
} from '@/lib/tauri-api';
import { asyncAction } from '@/lib/store-utils';

interface RegistryStore {
  registries: Registry[];
  activeRegistry: Registry | null;
  resources: Resource[];
  loading: boolean;
  syncing: boolean;
  error: string | null;
  activeTab: ResourceType;

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
  setActiveTab: (tab: ResourceType) => void;
  getFilteredResources: () => Resource[];

  registryPlugins: RegistryPlugin[];
  pluginResources: Record<string, Resource[]>;
  expandedPlugins: Set<string>;

  loadRegistryPlugins: (registryId: string) => Promise<void>;
  loadPluginResources: (pluginId: string) => Promise<void>;
  togglePluginExpanded: (pluginId: string) => void;
  installPluginToProject: (pluginId: string, projectId: string) => Promise<void>;
  installPluginToGlobal: (pluginId: string) => Promise<void>;

  resourceInstallStatus: Record<string, Record<string, ResourceLink[]>>;
  loadResourceInstallStatus: (pluginId: string) => Promise<void>;
  installResourceToProject: (resourceId: string, projectId: string, pluginId: string) => Promise<void>;
  installResourceToGlobal: (resourceId: string, pluginId: string) => Promise<void>;
  uninstallResource: (linkIds: string[], pluginId: string) => Promise<void>;
  updateInstalledResource: (resourceId: string, pluginId: string) => Promise<void>;
  retainAsLibrary: (resourceId: string, pluginId: string) => Promise<void>;
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
  resourceInstallStatus: {},

  loadRegistries: async () => {
    const registries = await asyncAction(set, 'loading', listRegistries);
    if (registries) set({ registries });
  },

  addRegistry: async (name: string, url: string, readonly: boolean) => {
    set({ loading: true, error: null });
    try {
      const registry = await apiAddRegistry(name, url, readonly);
      const registries = await listRegistries();
      set({ registries, loading: false });
      return registry;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  removeRegistry: async (id: string) => {
    await asyncAction(set, 'loading', async () => {
      await apiRemoveRegistry(id);
      const registries = await listRegistries();
      set({ registries, activeRegistry: null });
    });
  },

  syncRegistry: async (id: string) => {
    await asyncAction(set, 'syncing', async () => {
      await apiSyncRegistry(id);
      const registries = await listRegistries();
      const activeRegistry = registries.find((r) => r.id === id) ?? null;
      set({ registries, activeRegistry });
    });
  },

  syncAll: async () => {
    const registries = await asyncAction(set, 'syncing', syncAllRegistries);
    if (registries) set({ registries });
  },

  pushRegistry: async (id: string, message: string) => {
    await asyncAction(set, 'syncing', async () => {
      await apiPushRegistry(id, message);
      const registries = await listRegistries();
      set({ registries });
    });
  },

  checkUpdates: async () => {
    try {
      const registries = await checkRegistryUpdates();
      set({ registries });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadResources: async (registryId: string, resourceType?: ResourceType) => {
    const resources = await asyncAction(set, 'loading', () => listRegistryResources(registryId, resourceType));
    if (resources) set({ resources });
  },

  publishToRegistry: async (resourceId: string, registryId: string) => {
    const resource = await apiPublishToRegistry(resourceId, registryId);
    return resource;
  },

  selectRegistry: (registry: Registry | null) => {
    set({ activeRegistry: registry, resources: [] });
  },

  setActiveTab: (tab: ResourceType) => {
    set({ activeTab: tab });
  },

  getFilteredResources: () => {
    const { resources, activeTab } = get();
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

  loadResourceInstallStatus: async (pluginId: string) => {
    try {
      const status = await getPluginResourcesInstallStatus(pluginId);
      set((state) => ({
        resourceInstallStatus: { ...state.resourceInstallStatus, [pluginId]: status },
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  installResourceToProject: async (resourceId: string, projectId: string, pluginId: string) => {
    try {
      set({ syncing: true });
      await apiInstallResourceToProject(resourceId, projectId);
      const status = await getPluginResourcesInstallStatus(pluginId);
      set((state) => ({
        resourceInstallStatus: { ...state.resourceInstallStatus, [pluginId]: status },
        syncing: false,
      }));
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },

  installResourceToGlobal: async (resourceId: string, pluginId: string) => {
    try {
      set({ syncing: true });
      await apiInstallResourceToGlobal(resourceId);
      const status = await getPluginResourcesInstallStatus(pluginId);
      set((state) => ({
        resourceInstallStatus: { ...state.resourceInstallStatus, [pluginId]: status },
        syncing: false,
      }));
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },

  uninstallResource: async (linkIds: string[], pluginId: string) => {
    try {
      set({ syncing: true });
      await apiUninstallResource(linkIds);
      const status = await getPluginResourcesInstallStatus(pluginId);
      set((state) => ({
        resourceInstallStatus: { ...state.resourceInstallStatus, [pluginId]: status },
        syncing: false,
      }));
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },

  updateInstalledResource: async (resourceId: string, pluginId: string) => {
    try {
      set({ syncing: true });
      await apiUpdateInstalledResource(resourceId);
      const status = await getPluginResourcesInstallStatus(pluginId);
      const resources = await getRegistryPluginResources(pluginId);
      set((state) => ({
        resourceInstallStatus: { ...state.resourceInstallStatus, [pluginId]: status },
        pluginResources: { ...state.pluginResources, [pluginId]: resources },
        syncing: false,
      }));
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },

  retainAsLibrary: async (resourceId: string, pluginId: string) => {
    try {
      set({ syncing: true });
      await apiRetainAsLibrary(resourceId);
      const status = await getPluginResourcesInstallStatus(pluginId);
      const resources = await getRegistryPluginResources(pluginId);
      set((state) => ({
        resourceInstallStatus: { ...state.resourceInstallStatus, [pluginId]: status },
        pluginResources: { ...state.pluginResources, [pluginId]: resources },
        syncing: false,
      }));
    } catch (e) {
      set({ error: String(e), syncing: false });
    }
  },
}));
