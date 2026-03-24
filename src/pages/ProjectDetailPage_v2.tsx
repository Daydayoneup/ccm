import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { ArrowLeft, Pin, PinOff, Plus, RefreshCw, Terminal } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { PageHeader, PageShell, PanelSection, ToolbarRow, InlineStatus } from '@/components/layout/PageShell';
import { EnvVarTable } from '@/components/shared/EnvVarTable';
import { McpServerList } from '@/components/shared/McpServerList';
import { PermissionsList } from '@/components/shared/PermissionsList';
import { ResourceTable } from '@/components/shared/ResourceTable';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { FileExplorer } from '@/components/files/FileExplorer';
import { useI18n } from '@/i18n/provider';
import { useGlobalStore } from '@/stores/global-store';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import type { MergedEnvVar, ResourceType } from '@/types/v2';

const resourceTemplates: Record<ResourceType, (name: string) => string> = {
  skill: (name) => `---\nname: ${name}\ndescription: \n---\n\n# ${name}\n\n`,
  agent: (name) => `# ${name} Agent\n\nYou are...\n`,
  rule: (name) => `# ${name}\n\n`,
  hook: () => `{\n  "hooks": {}\n}\n`,
  mcp_server: () => `{\n  "mcpServers": {}\n}\n`,
  command: (name) => `# ${name}\n\nUsage: /${name}\n\n`,
};

export function ProjectDetailPageV2() {
  const { t } = useI18n();
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

  const project = projects.find((item) => item.id === id);
  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [creating, setCreating] = useState(false);
  const [projectEnvVars, setProjectEnvVars] = useState<MergedEnvVar[]>([]);
  const [rescanning, setRescanning] = useState(false);

  useEffect(() => {
    if (id) {
      invoke<MergedEnvVar[]>('list_merged_env_vars', { projectId: id })
        .then(setProjectEnvVars)
        .catch((err) => console.error('Failed to load env vars:', err));
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
  }, [id, loadGlobalMcpServers, loadGlobalResources, loadProjectMcpServers, loadProjectResources]);

  const filteredResources = projectResources.filter((resource) => resource.resource_type === activeTab);
  const filteredGlobalResources = activeTab !== 'mcp' && activeTab !== 'permissions'
    ? globalResources.filter((resource) => resource.resource_type === activeTab)
    : [];
  const mergedResources = [...filteredResources, ...filteredGlobalResources];
  const mergedMcpServers = [...projectMcpServers, ...globalMcpServers];
  const globalMcpServerIds = new Set(globalMcpServers.map((server) => server.id));

  const resourceCounts: Record<string, number> = {};
  [...projectResources, ...globalResources].forEach((resource) => {
    resourceCounts[resource.resource_type] = (resourceCounts[resource.resource_type] ?? 0) + 1;
  });

  const handleCreate = async () => {
    if (!id || !newName.trim() || activeTab === 'mcp') return;
    setCreating(true);
    try {
      const resourceType = activeTab as ResourceType;
      const content = resourceTemplates[resourceType](newName.trim());
      const resource = await createResource(id, resourceType, newName.trim(), content);
      setCreateOpen(false);
      setNewName('');
      const filePath = resource.source_path;
      const extra = resource.resource_type === 'skill'
        ? `&resource_id=${resource.id}&type=skill&scope=project`
        : '';
      navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
    } catch (error) {
      console.error('Failed to create resource:', error);
    } finally {
      setCreating(false);
    }
  };

  const handleRescan = async () => {
    if (!id) return;
    setRescanning(true);
    try {
      await rescanProject(id);
    } catch (error) {
      console.error('Failed to rescan:', error);
    } finally {
      setRescanning(false);
    }
  };

  if (!project && projects.length > 0) {
    return (
      <PageShell>
        <InlineStatus tone="danger">{t('projectDetail.missing')}</InlineStatus>
      </PageShell>
    );
  }

  return (
    <PageShell className="gap-5">
      <PageHeader
        eyebrow={t('nav.projects')}
        title={project?.name ?? t('common.loading')}
        description={project?.path}
        actions={
          <>
            <Button variant="outline" size="sm" className="rounded-md" onClick={() => navigate('/projects')}>
              <ArrowLeft className="size-3.5" />
              {t('common.back')}
            </Button>
            {project ? (
              <>
                <Button
                  variant="ghost"
                  size="icon"
                  className="rounded-sm"
                  onClick={() => togglePin(project.id)}
                  title={project.pinned === 1 ? t('projects.unpin') : t('projects.pin')}
                >
                  {project.pinned === 1 ? <PinOff className="size-4" /> : <Pin className="size-4" />}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="rounded-md"
                  onClick={handleRescan}
                  disabled={rescanning}
                >
                  <RefreshCw className={`size-3.5 ${rescanning ? 'animate-spin' : ''}`} />
                  {t('projectDetail.rescan')}
                </Button>
                <Button
                  size="sm"
                  className="rounded-md"
                  onClick={() => invoke('launch_claude_in_terminal', { projectPath: project.path, projectId: project.id })}
                >
                  <Terminal className="size-3.5" />
                  {t('projectDetail.launchShell')}
                </Button>
              </>
            ) : null}
          </>
        }
      />

      <ToolbarRow>
        <ResourceTypeTabs
          activeTab={activeTab as string}
          onTabChange={setActiveTab as (tab: string) => void}
          includeMcp
          includePermissions
          includeEnv
          includeFiles
          counts={resourceCounts}
        />
        {activeTab !== 'mcp' && activeTab !== 'permissions' && activeTab !== 'env' && activeTab !== 'files' ? (
          <Button size="sm" className="shrink-0 rounded-md" onClick={() => setCreateOpen(true)}>
            <Plus className="size-3.5" />
            {t('projectDetail.createResource', { type: t(`resourceTypes.${activeTab}`) })}
          </Button>
        ) : null}
      </ToolbarRow>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>{t('projectDetail.createDialogTitle', { type: t(`resourceTypes.${activeTab}`) })}</DialogTitle>
          </DialogHeader>
          <div className="py-2">
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t('projectDetail.createPlaceholder', { type: activeTab })}
              autoFocus
              onKeyDown={(e) => {
                if (e.key === 'Enter' && newName.trim()) handleCreate();
              }}
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setCreateOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button size="sm" onClick={handleCreate} disabled={!newName.trim() || creating}>
              {creating ? t('projectDetail.creating') : t('common.create')}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {error ? <InlineStatus tone="danger">{error}</InlineStatus> : null}

      <PanelSection>
        {activeTab === 'files' ? (
          project ? <FileExplorer projectPath={project.path} /> : null
        ) : activeTab === 'env' ? (
          <EnvVarTable
            vars={projectEnvVars}
            onAdd={async (key, value) => {
              if (!id) return;
              await invoke('set_env_var', { projectId: id, key, value });
              const vars = await invoke<MergedEnvVar[]>('list_merged_env_vars', { projectId: id });
              setProjectEnvVars(vars);
            }}
            onDelete={async (envVarId) => {
              if (!id) return;
              await invoke('delete_env_var', { id: envVarId });
              const vars = await invoke<MergedEnvVar[]>('list_merged_env_vars', { projectId: id });
              setProjectEnvVars(vars);
            }}
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
            emptyMessage={t('projectDetail.mcpEmpty')}
            emptyHint={t('projectDetail.mcpHint')}
          />
        ) : resourcesLoading ? (
          <div className="flex items-center justify-center py-16 text-muted-foreground">
            {t('projectDetail.loadingResources')}
          </div>
        ) : (
          <ResourceTable
            resources={mergedResources}
            onDelete={(resourceId) => deleteResource(resourceId, id)}
            onBackup={async (resourceId) => {
              try {
                await publishToLibrary(resourceId, false);
              } catch (error) {
                console.error('Failed to publish:', error);
              }
            }}
          />
        )}
      </PanelSection>
    </PageShell>
  );
}
