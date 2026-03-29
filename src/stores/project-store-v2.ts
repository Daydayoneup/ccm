import { create } from 'zustand';
import {
  listProjectsV2, registerProjectV2, removeProjectV2, scanAndDiscoverProjects,
  discoverClaudeProjects, listProjectResources, createProjectResource,
  deleteProjectResource, publishToLibrary, installFromLibrary, rescanProject,
  toggleProjectPin,
} from '@/lib/tauri-api';
import type { Resource, Project, ResourceType, DiscoveredProject } from '@/types/v2';

interface ProjectStore {
  projects: Project[];
  selectedProject: Project | null;
  projectResources: Resource[];
  discoveredProjects: Project[];
  claudeDiscoveredProjects: DiscoveredProject[];
  loading: boolean;
  resourcesLoading: boolean;
  error: string | null;
  activeTab: ResourceType | 'permissions' | 'env' | 'files';

  // Project actions
  loadProjects: () => Promise<void>;
  registerProject: (path: string) => Promise<Project>;
  removeProject: (id: string, deleteFromDisk?: boolean) => Promise<void>;
  selectProject: (project: Project | null) => void;
  discoverProjects: (directories: string[]) => Promise<void>;
  discoverFromClaude: () => Promise<void>;

  // Project resource actions
  loadProjectResources: (projectId: string, resourceType?: ResourceType) => Promise<void>;
  createResource: (projectId: string, resourceType: ResourceType, name: string, content: string) => Promise<Resource>;
  deleteResource: (resourceId: string, projectId?: string) => Promise<void>;
  publishToLibrary: (resourceId: string, replaceWithSymlink: boolean) => Promise<Resource>;
  installFromLibrary: (libraryResourceId: string, projectId: string, linkType: string) => Promise<void>;
  rescanProject: (projectId: string) => Promise<{ added: number; removed: number }>;
  togglePin: (projectId: string) => Promise<void>;

  setActiveTab: (tab: ResourceType | 'permissions' | 'env' | 'files') => void;
  getFilteredResources: () => Resource[];
}

export const useProjectStoreV2 = create<ProjectStore>((set, get) => ({
  projects: [],
  selectedProject: null,
  projectResources: [],
  discoveredProjects: [],
  claudeDiscoveredProjects: [],
  loading: false,
  resourcesLoading: false,
  error: null,
  activeTab: 'files',

  loadProjects: async () => {
    set({ loading: true, error: null });
    try {
      const projects = await listProjectsV2();
      set({ projects, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  registerProject: async (path: string) => {
    const project = await registerProjectV2(path);
    await get().loadProjects();
    return project;
  },

  removeProject: async (id: string, deleteFromDisk = false) => {
    const result = await removeProjectV2(id, deleteFromDisk);
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
    set({ selectedProject: project, projectResources: [] });
  },

  discoverProjects: async (directories: string[]) => {
    try {
      const discovered = await scanAndDiscoverProjects(directories);
      set({ discoveredProjects: discovered });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  discoverFromClaude: async () => {
    try {
      const discovered = await discoverClaudeProjects();
      set({ claudeDiscoveredProjects: discovered });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadProjectResources: async (projectId: string, resourceType?: ResourceType) => {
    set({ resourcesLoading: true });
    try {
      const resources = await listProjectResources(projectId, resourceType);
      set({ projectResources: resources, resourcesLoading: false });
    } catch (e) {
      set({ error: String(e), resourcesLoading: false });
    }
  },

  createResource: async (projectId: string, resourceType: ResourceType, name: string, content: string) => {
    const resource = await createProjectResource(projectId, resourceType, name, content);
    await get().loadProjectResources(projectId);
    return resource;
  },

  deleteResource: async (resourceId: string, projectId?: string) => {
    try {
      await deleteProjectResource(resourceId);
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
    const resource = await publishToLibrary(resourceId, replaceWithSymlink);
    return resource;
  },

  installFromLibrary: async (libraryResourceId: string, projectId: string, linkType: string) => {
    await installFromLibrary(libraryResourceId, projectId, linkType);
    await get().loadProjectResources(projectId);
  },

  rescanProject: async (projectId: string) => {
    const result = await rescanProject(projectId);
    await get().loadProjectResources(projectId);
    return result;
  },

  togglePin: async (projectId: string) => {
    const updated: Project = await toggleProjectPin(projectId);
    set((state) => ({
      projects: state.projects.map((p) => p.id === updated.id ? updated : p),
      selectedProject: state.selectedProject?.id === updated.id ? updated : state.selectedProject,
    }));
  },

  setActiveTab: (tab: ResourceType | 'permissions' | 'env' | 'files') => {
    set({ activeTab: tab });
  },

  getFilteredResources: () => {
    const { projectResources, activeTab } = get();
    return projectResources.filter((r) => r.resource_type === activeTab);
  },
}));
