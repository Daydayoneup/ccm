import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Resource, Project, ResourceType, McpServer, DiscoveredProject } from '@/types/v2';

interface ProjectStore {
  projects: Project[];
  selectedProject: Project | null;
  projectResources: Resource[];
  projectMcpServers: McpServer[];
  discoveredProjects: Project[];
  claudeDiscoveredProjects: DiscoveredProject[];
  loading: boolean;
  resourcesLoading: boolean;
  error: string | null;
  activeTab: ResourceType | 'mcp' | 'permissions' | 'env' | 'files';

  // Project actions
  loadProjects: () => Promise<void>;
  registerProject: (path: string) => Promise<Project>;
  removeProject: (id: string, deleteFromDisk?: boolean) => Promise<void>;
  selectProject: (project: Project | null) => void;
  discoverProjects: (directories: string[]) => Promise<void>;
  discoverFromClaude: () => Promise<void>;

  // Project resource actions
  loadProjectResources: (projectId: string, resourceType?: ResourceType) => Promise<void>;
  loadProjectMcpServers: (projectId: string) => Promise<void>;
  createResource: (projectId: string, resourceType: ResourceType, name: string, content: string) => Promise<Resource>;
  deleteResource: (resourceId: string, projectId?: string) => Promise<void>;
  publishToLibrary: (resourceId: string, replaceWithSymlink: boolean) => Promise<Resource>;
  installFromLibrary: (libraryResourceId: string, projectId: string, linkType: string) => Promise<void>;
  rescanProject: (projectId: string) => Promise<{ added: number; removed: number }>;
  togglePin: (projectId: string) => Promise<void>;

  setActiveTab: (tab: ResourceType | 'mcp' | 'permissions' | 'env' | 'files') => void;
  getFilteredResources: () => Resource[];
}

export const useProjectStoreV2 = create<ProjectStore>((set, get) => ({
  projects: [],
  selectedProject: null,
  projectResources: [],
  projectMcpServers: [],
  discoveredProjects: [],
  claudeDiscoveredProjects: [],
  loading: false,
  resourcesLoading: false,
  error: null,
  activeTab: 'files',

  loadProjects: async () => {
    set({ loading: true, error: null });
    try {
      const projects = await invoke<Project[]>('list_projects_v2');
      set({ projects, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  registerProject: async (path: string) => {
    const project = await invoke<Project>('register_project_v2', { path });
    await get().loadProjects();
    return project;
  },

  removeProject: async (id: string, deleteFromDisk = false) => {
    const result = await invoke<{ warnings: string[] }>('remove_project_v2', { id, deleteFromDisk });
    if (result.warnings.length > 0) {
      console.warn('Project removal warnings:', result.warnings);
    }
    const { selectedProject } = get();
    if (selectedProject?.id === id) {
      set({ selectedProject: null, projectResources: [] });
    }
    await get().loadProjects();
  },

  selectProject: (project: Project | null) => {
    set({ selectedProject: project, projectResources: [], projectMcpServers: [] });
  },

  discoverProjects: async (directories: string[]) => {
    try {
      const discovered = await invoke<Project[]>('scan_and_discover_projects', { directories });
      set({ discoveredProjects: discovered });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  discoverFromClaude: async () => {
    try {
      const discovered = await invoke<DiscoveredProject[]>('discover_claude_projects');
      set({ claudeDiscoveredProjects: discovered });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadProjectResources: async (projectId: string, resourceType?: ResourceType) => {
    set({ resourcesLoading: true });
    try {
      const resources = await invoke<Resource[]>('list_project_resources', {
        projectId,
        resourceType: resourceType ?? null,
      });
      set({ projectResources: resources, resourcesLoading: false });
    } catch (e) {
      set({ error: String(e), resourcesLoading: false });
    }
  },

  loadProjectMcpServers: async (projectId: string) => {
    try {
      const servers = await invoke<McpServer[]>('list_project_mcp_servers', { projectId });
      set({ projectMcpServers: servers });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createResource: async (projectId: string, resourceType: ResourceType, name: string, content: string) => {
    const resource = await invoke<Resource>('create_project_resource', {
      projectId,
      resourceType,
      name,
      content,
    });
    await get().loadProjectResources(projectId);
    return resource;
  },

  deleteResource: async (resourceId: string, projectId?: string) => {
    try {
      await invoke<void>('delete_project_resource', { resourceId });
    } catch (e) {
      set({ error: String(e) });
      return;  // Don't reload on error
    }
    const resolvedId = projectId ?? get().selectedProject?.id;
    if (resolvedId) {
      await get().loadProjectResources(resolvedId);
    }
  },

  publishToLibrary: async (resourceId: string, replaceWithSymlink: boolean) => {
    const resource = await invoke<Resource>('publish_to_library', {
      resourceId,
      replaceWithSymlink,
    });
    return resource;
  },

  installFromLibrary: async (libraryResourceId: string, projectId: string, linkType: string) => {
    await invoke<void>('install_from_library', {
      libraryResourceId,
      projectId,
      linkType,
    });
    await get().loadProjectResources(projectId);
  },

  rescanProject: async (projectId: string) => {
    const result = await invoke<{ added: number; removed: number }>('rescan_project', { projectId });
    await get().loadProjectResources(projectId);
    return result;
  },

  togglePin: async (projectId: string) => {
    const updated: Project = await invoke('toggle_project_pin', { projectId });
    set((state) => ({
      projects: state.projects.map((p) => p.id === updated.id ? updated : p),
      selectedProject: state.selectedProject?.id === updated.id ? updated : state.selectedProject,
    }));
  },

  setActiveTab: (tab: ResourceType | 'mcp' | 'permissions' | 'env' | 'files') => {
    set({ activeTab: tab });
  },

  getFilteredResources: () => {
    const { projectResources, activeTab } = get();
    return projectResources.filter((r) => r.resource_type === activeTab);
  },
}));
