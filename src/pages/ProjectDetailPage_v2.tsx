import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { navigateToResource } from '@/lib/navigation';
import { launchClaudeInTerminal, setEnvVar, deleteEnvVar, listMergedEnvVars } from '@/lib/tauri-api';
import { ArrowLeft, Pin, PinOff, Plus, RefreshCw, Terminal } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { PageShell, PanelSection, ToolbarRow, InlineStatus } from '@/components/layout/PageShell';
import { EnvVarTable } from '@/components/shared/EnvVarTable';
import { McpServerList } from '@/components/shared/McpServerList';
import { PermissionsList } from '@/components/shared/PermissionsList';
import { CreateResourceDialog } from '@/components/shared/CreateResourceDialog';
import { ResourceTable } from '@/components/shared/ResourceTable';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { FileExplorer } from '@/components/files/FileExplorer';
import { useI18n } from '@/i18n/provider';
import { useGlobalStore } from '@/stores/global-store';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import type { MergedEnvVar, ResourceType } from '@/types/v2';

export function ProjectDetailPageV2() {
  const { t } = useI18n();
  const { projectId: id } = useParams<{ projectId: string }>();
  const navigate = useNavigate();
  const {
    projects,
    projectResources,
    resourcesLoading,
    error,
    activeTab,
    loadProjects,
    loadProjectResources,
    createResource,
    deleteResource,
    publishToLibrary,
    rescanProject,
    setActiveTab,
    togglePin,
  } = useProjectStoreV2();
  const {
    resources: globalResources,
    loadResources: loadGlobalResources,
  } = useGlobalStore();

  const project = projects.find((item) => item.id === id);
  const [createOpen, setCreateOpen] = useState(false);
  const [projectEnvVars, setProjectEnvVars] = useState<MergedEnvVar[]>([]);
  const [rescanning, setRescanning] = useState(false);

  useEffect(() => {
    if (id) {
      listMergedEnvVars(id)
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
    }
    loadGlobalResources();
  }, [id, loadGlobalResources, loadProjectResources]);

  const filteredResources = projectResources.filter((resource) => resource.resource_type === activeTab);
  const filteredGlobalResources = activeTab !== 'permissions'
    ? globalResources.filter((resource) => resource.resource_type === activeTab)
    : [];
  const mergedResources = [...filteredResources, ...filteredGlobalResources];

  const resourceCounts: Record<string, number> = {};
  [...projectResources, ...globalResources].forEach((resource) => {
    resourceCounts[resource.resource_type] = (resourceCounts[resource.resource_type] ?? 0) + 1;
  });

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
      {/* Header: back arrow + project info + actions */}
      <div className="flex items-center gap-4">
        <button
          onClick={() => navigate('/projects')}
          className="group flex size-8 shrink-0 items-center justify-center rounded-lg border border-border/60 bg-card/80 text-muted-foreground transition-all hover:border-primary/30 hover:bg-primary/8 hover:text-primary"
        >
          <ArrowLeft className="size-4 transition-transform group-hover:-translate-x-0.5" />
        </button>

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2.5">
            <h1 className="truncate text-xl font-semibold tracking-tight">
              {project?.name ?? t('common.loading')}
            </h1>
            {project ? (
              <button
                onClick={() => togglePin(project.id)}
                className="shrink-0 text-muted-foreground/50 transition-colors hover:text-primary"
                title={project.pinned === 1 ? t('projects.unpin') : t('projects.pin')}
              >
                {project.pinned === 1 ? <PinOff className="size-3.5" /> : <Pin className="size-3.5" />}
              </button>
            ) : null}
          </div>
          <p className="mt-0.5 truncate text-xs text-muted-foreground/70">{project?.path}</p>
        </div>

        {project ? (
          <div className="flex shrink-0 items-center gap-2">
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
              onClick={() => launchClaudeInTerminal(project.path)}
            >
              <Terminal className="size-3.5" />
              {t('projectDetail.launchShell')}
            </Button>
          </div>
        ) : null}
      </div>

      {/* Tabs + create button */}
      <ToolbarRow>
        <ResourceTypeTabs
          activeTab={activeTab as string}
          onTabChange={setActiveTab as (tab: string) => void}
          includePermissions
          includeEnv
          includeFiles
          counts={resourceCounts}
        />
        {activeTab !== 'mcp_server' && activeTab !== 'permissions' && activeTab !== 'env' && activeTab !== 'files' ? (
          <Button size="sm" className="shrink-0 rounded-md" onClick={() => setCreateOpen(true)}>
            <Plus className="size-3.5" />
            {t('projectDetail.createResource', { type: t(`resourceTypes.${activeTab}`) })}
          </Button>
        ) : null}
      </ToolbarRow>

      <CreateResourceDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        resourceType={activeTab as ResourceType}
        onSubmit={async (type, name, content) => {
          if (!id) throw new Error('No project ID');
          const resource = await createResource(id, type, name, content);
          return resource;
        }}
        onCreated={(resource) => navigateToResource(navigate, resource)}
      />

      {error ? <InlineStatus tone="danger">{error}</InlineStatus> : null}

      {/* Content area */}
      <PanelSection>
        {activeTab === 'files' ? (
          project ? <FileExplorer projectPath={project.path} /> : null
        ) : activeTab === 'env' ? (
          <EnvVarTable
            vars={projectEnvVars}
            onAdd={async (key, value) => {
              if (!id) return;
              await setEnvVar(id, key, value);
              const vars = await listMergedEnvVars(id);
              setProjectEnvVars(vars);
            }}
            onDelete={async (envVarId) => {
              if (!id) return;
              await deleteEnvVar(envVarId);
              const vars = await listMergedEnvVars(id);
              setProjectEnvVars(vars);
            }}
            readonlyScope="global"
            onReadonlyClick={() => navigate('/settings')}
          />
        ) : activeTab === 'permissions' ? (
          id ? <PermissionsList projectId={id} /> : null
        ) : activeTab === 'mcp_server' ? (
          <McpServerList
            resources={mergedResources}
            emptyMessage={t('projectDetail.mcpEmpty')}
            emptyHint={t('projectDetail.mcpHint')}
            projectId={id}
            onRefresh={() => { if (id) loadProjectResources(id); loadGlobalResources(); }}
          />
        ) : resourcesLoading ? (
          <div className="flex items-center justify-center py-16 text-muted-foreground">
            {t('projectDetail.loadingResources')}
          </div>
        ) : (
          <ResourceTable
            resources={mergedResources}
            onDelete={(resourceId) => deleteResource(resourceId, id)}
            onBackup={async (resourceId, replaceWithLink) => {
              try {
                await publishToLibrary(resourceId, replaceWithLink);
                if (id) await loadProjectResources(id);
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
