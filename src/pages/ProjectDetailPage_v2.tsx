import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import { useGlobalStore } from '@/stores/global-store';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { ResourceTable } from '@/components/shared/ResourceTable';
import { McpServerList } from '@/components/shared/McpServerList';
import { PermissionsList } from '@/components/shared/PermissionsList';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ArrowLeft, Plus, Terminal, RefreshCw, Pin, PinOff } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { EnvVarTable } from '@/components/shared/EnvVarTable';
import { FileExplorer } from '@/components/files/FileExplorer';
import type { ResourceType, MergedEnvVar } from '@/types/v2';

const resourceTemplates: Record<ResourceType, (name: string) => string> = {
  skill: (name) => `---\nname: ${name}\ndescription: \n---\n\n# ${name}\n\n`,
  agent: (name) => `# ${name} Agent\n\nYou are...\n`,
  rule: (name) => `# ${name}\n\n`,
  hook: () => `{\n  "hooks": {}\n}\n`,
  mcp_server: () => `{\n  "mcpServers": {}\n}\n`,
  command: (name) => `# ${name}\n\nUsage: /${name}\n\n`,
};

const resourceTypeLabels: Record<ResourceType, string> = {
  skill: 'Skill',
  agent: 'Agent',
  rule: 'Rule',
  hook: 'Hook',
  mcp_server: 'MCP Server',
  command: 'Command',
};

export function ProjectDetailPageV2() {
  const { projectId: id } = useParams<{ projectId: string }>();
  const navigate = useNavigate();
  const {
    projects,
    projectResources,
    resourcesLoading,
    projectMcpServers,
    error,
    activeTab,
    loadProjects,
    loadProjectResources,
    loadProjectMcpServers,
    createResource,
    deleteResource,
    publishToLibrary,
    rescanProject,
    setActiveTab,
    togglePin,
  } = useProjectStoreV2();

  const {
    resources: globalResources,
    globalMcpServers,
    loadResources: loadGlobalResources,
    loadGlobalMcpServers,
  } = useGlobalStore();

  const project = projects.find((p) => p.id === id);

  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [creating, setCreating] = useState(false);
  const [projectEnvVars, setProjectEnvVars] = useState<MergedEnvVar[]>([]);
  const [rescanning, setRescanning] = useState(false);

  useEffect(() => {
    if (id) {
      loadProjectEnvVars(id);
    }
  }, [id]);

  useEffect(() => {
    if (projects.length === 0) {
      loadProjects();
    }
  }, [projects.length, loadProjects]);

  useEffect(() => {
    if (id) {
      loadProjectResources(id);
      loadProjectMcpServers(id);
    }
    loadGlobalResources();
    loadGlobalMcpServers();
  }, [id, loadProjectResources, loadProjectMcpServers, loadGlobalResources, loadGlobalMcpServers]);

  const filteredResources = projectResources.filter((r) => r.resource_type === activeTab);
  const filteredGlobalResources = activeTab !== 'mcp' && activeTab !== 'permissions'
    ? globalResources.filter((r) => r.resource_type === activeTab)
    : [];
  const mergedResources = [...filteredResources, ...filteredGlobalResources];
  const mergedMcpServers = [...projectMcpServers, ...globalMcpServers];
  const globalMcpServerIds = new Set(globalMcpServers.map((s) => s.id));

  // Compute counts per resource type for tab badges
  const allResources = [...projectResources, ...globalResources];
  const resourceCounts: Record<string, number> = {};
  for (const r of allResources) {
    resourceCounts[r.resource_type] = (resourceCounts[r.resource_type] ?? 0) + 1;
  }

  const loadProjectEnvVars = async (projectId: string) => {
    try {
      const vars = await invoke<MergedEnvVar[]>('list_merged_env_vars', { projectId });
      setProjectEnvVars(vars);
    } catch (err) {
      console.error('Failed to load env vars:', err);
    }
  };

  const handleAddProjectEnvVar = async (key: string, value: string) => {
    if (!id) return;
    await invoke('set_env_var', { projectId: id, key, value });
    await loadProjectEnvVars(id);
  };

  const handleDeleteProjectEnvVar = async (envVarId: string) => {
    if (!id) return;
    await invoke('delete_env_var', { id: envVarId });
    await loadProjectEnvVars(id);
  };

  const handleRescan = async () => {
    if (!id) return;
    setRescanning(true);
    try {
      await rescanProject(id);
    } catch (e) {
      console.error('Failed to rescan:', e);
    } finally {
      setRescanning(false);
    }
  };

  const handlePublish = async (resourceId: string) => {
    try {
      await publishToLibrary(resourceId, false);
    } catch (e) {
      console.error('Failed to publish:', e);
    }
  };

  const handleCreate = async () => {
    if (!id || !newName.trim() || activeTab === 'mcp') return;
    setCreating(true);
    try {
      const resourceType = activeTab as ResourceType;
      const content = resourceTemplates[resourceType](newName.trim());
      const resource = await createResource(id, resourceType, newName.trim(), content);
      setCreateOpen(false);
      setNewName('');
      // Navigate directly to editor
      const filePath = resource.resource_type === 'skill'
        ? `${resource.source_path}/SKILL.md`
        : resource.source_path;
      const extra = resource.resource_type === 'skill'
        ? `&resource_id=${resource.id}&type=skill`
        : '';
      navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
    } catch (e) {
      console.error('Failed to create resource:', e);
    } finally {
      setCreating(false);
    }
  };

  if (!project && projects.length > 0) {
    return (
      <div className="flex items-center justify-center p-8">
        <p className="text-muted-foreground">Project not found</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 p-8">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button
          variant="ghost"
          size="icon"
          className="shrink-0 rounded-lg"
          onClick={() => navigate('/projects')}
        >
          <ArrowLeft className="size-4" />
        </Button>
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-2xl font-bold tracking-tight">
            {project?.name ?? 'Loading...'}
          </h1>
          <p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">
            {project?.path}
          </p>
        </div>
        {project && (
          <div className="flex items-center gap-2">
            <button
              onClick={() => togglePin(project.id)}
              className="rounded-md p-2 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              title={project.pinned === 1 ? 'Unpin project' : 'Pin project'}
            >
              {project.pinned === 1 ? <PinOff className="size-4" /> : <Pin className="size-4" />}
            </button>
            <Button
              variant="ghost"
              size="icon"
              className="rounded-lg text-muted-foreground hover:text-primary"
              onClick={handleRescan}
              disabled={rescanning}
              title="Rescan project directory"
            >
              <RefreshCw className={`size-4 ${rescanning ? 'animate-spin' : ''}`} />
            </Button>
            <Button
              size="sm"
              className="gap-1.5 rounded-lg bg-primary/15 text-primary shadow-none hover:bg-primary hover:text-primary-foreground transition-all duration-200"
              onClick={() => invoke('launch_claude_in_terminal', { projectPath: project.path, projectId: project.id })}
              title="Launch Claude in Terminal"
            >
              <Terminal className="size-3.5" />
              启动 Shell
            </Button>
          </div>
        )}
      </div>

      {/* Tabs + Create button */}
      <div className="flex items-center justify-between gap-4">
        <ResourceTypeTabs
          activeTab={activeTab as string}
          onTabChange={setActiveTab as (tab: string) => void}
          includeMcp
          includePermissions
          includeEnv
          includeFiles
          counts={resourceCounts}
        />
        {activeTab !== 'mcp' && activeTab !== 'permissions' && activeTab !== 'env' && activeTab !== 'files' && (
          <Button
            size="sm"
            className="shrink-0 rounded-lg"
            onClick={() => setCreateOpen(true)}
          >
            <Plus className="mr-1.5 size-3.5" />
            New {resourceTypeLabels[activeTab as ResourceType]}
          </Button>
        )}
      </div>

      {/* Create dialog — name only, then navigate to editor */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>
              New {resourceTypeLabels[activeTab as ResourceType] ?? activeTab}
            </DialogTitle>
          </DialogHeader>
          <div className="py-2">
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={`my-${activeTab}`}
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter' && newName.trim()) handleCreate();
              }}
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setCreateOpen(false)}>
              Cancel
            </Button>
            <Button size="sm" onClick={handleCreate} disabled={!newName.trim() || creating}>
              {creating ? 'Creating...' : 'Create'}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Error */}
      {error && (
        <div className="rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* Content */}
      {activeTab === 'files' ? (
        project ? <FileExplorer projectPath={project.path} /> : null
      ) : activeTab === 'env' ? (
        <EnvVarTable
          vars={projectEnvVars}
          onAdd={handleAddProjectEnvVar}
          onDelete={handleDeleteProjectEnvVar}
          readonlyScope="global"
          onReadonlyClick={() => navigate('/settings')}
        />
      ) : activeTab === 'permissions' ? (
        id ? <PermissionsList projectId={id} /> : null
      ) : activeTab === 'mcp' ? (
        <McpServerList
          servers={mergedMcpServers}
          globalServerIds={globalMcpServerIds}
          onServerClick={() => navigate('/global?tab=mcp')}
          emptyMessage="No MCP servers configured for this project."
          emptyHint="Add a .mcp.json file to the project root to define MCP servers."
        />
      ) : resourcesLoading ? (
        <div className="flex items-center justify-center py-16 text-muted-foreground">
          Loading resources...
        </div>
      ) : (
        <ResourceTable
          resources={mergedResources}
          onDelete={(resourceId, _deleteFromDisk) => deleteResource(resourceId, id)}
          onBackup={handlePublish}
        />
      )}
    </div>
  );
}
