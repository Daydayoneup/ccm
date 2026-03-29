import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { navigateToResource } from '@/lib/navigation';
import { Package, Plus, Trash2 } from 'lucide-react';
import { ResourceCard } from '@/components/shared/ResourceCard';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { DeleteConfirmDialog } from '@/components/shared/DeleteConfirmDialog';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { CreateResourceDialog } from '@/components/shared/CreateResourceDialog';
import { DeployToGlobalDialog } from '@/components/library/DeployToGlobalDialog';
import { InstallToProjectDialog } from '@/components/library/InstallToProjectDialog';
import { LinkHealthBadge } from '@/components/library/LinkHealthBadge';
import { LibraryPluginTab } from '@/components/library/LibraryPluginTab';
import { AddToPluginDialog } from '@/components/library/AddToPluginDialog';
import { LibrarySourcePanel } from '@/components/library/LibrarySourcePanel';
import { InstallPluginToGlobalDialog } from '@/components/library/InstallPluginToGlobalDialog';
import { InstallPluginToProjectDialog } from '@/components/library/InstallPluginToProjectDialog';
import { EmptyState, InlineStatus, PageHeader, PageShell, PanelSection, ToolbarRow } from '@/components/layout/PageShell';
import { useI18n } from '@/i18n/provider';
import { useRegistryStore } from '@/stores/registry-store';
import { useLibraryStore } from '@/stores/library-store-v2';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import type { LinkType, Resource } from '@/types/v2';
import { RegistryPluginList } from '@/components/registry/RegistryPluginList';
import { updateInstalledResource as apiUpdateInstalledResource, retainAsLibrary as apiRetainAsLibrary } from '@/lib/tauri-api';

export function LibraryPage_v2() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const {
    loading,
    error,
    activeTab,
    linkHealth,
    loadResources,
    createResource,
    deleteResource,
    installToProject,
    deployToGlobal,
    checkLinkHealth,
    setActiveTab,
    getFilteredResources,
    selectedSource,
    installFilter,
    setInstallFilter,
  } = useLibraryStore();
  const { registries, installPluginToProject, installPluginToGlobal } = useRegistryStore();
  const { projects, loadProjects } = useProjectStoreV2();

  const [installDialogOpen, setInstallDialogOpen] = useState(false);
  const [deployDialogOpen, setDeployDialogOpen] = useState(false);
  const [selectedResourceId, setSelectedResourceId] = useState<string | null>(null);
  const [selectedResourceName, setSelectedResourceName] = useState('');
  const [addToPluginResourceId, setAddToPluginResourceId] = useState<string | null>(null);
  const [pageTab, setPageTab] = useState<'resources' | 'plugin-packs'>('resources');
  const [projectInstallDialogOpen, setProjectInstallDialogOpen] = useState(false);
  const [projectInstallPluginId, setProjectInstallPluginId] = useState<string | null>(null);
  const [projectInstallPluginName, setProjectInstallPluginName] = useState('');
  const [globalInstallDialogOpen, setGlobalInstallDialogOpen] = useState(false);
  const [globalInstallPluginId, setGlobalInstallPluginId] = useState<string | null>(null);
  const [globalInstallPluginName, setGlobalInstallPluginName] = useState('');
  const [pendingDelete, setPendingDelete] = useState<Resource | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [resourceInstall, setResourceInstall] = useState<{
    mode: 'project' | 'global';
    resourceId: string;
    resourceName: string;
    pluginId: string;
  } | null>(null);

  useEffect(() => {
    loadResources();
  }, [loadResources]);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  const projectNames = useMemo(() => {
    const map: Record<string, string> = {};
    projects.forEach((p) => { map[p.id] = p.name; });
    return map;
  }, [projects]);

  const handleInstallResource = (resourceId: string, resourceName: string, pluginId: string) => {
    setResourceInstall({ mode: 'project', resourceId, resourceName, pluginId });
  };

  const handleInstallResourceToGlobal = (resourceId: string, resourceName: string, pluginId: string) => {
    setResourceInstall({ mode: 'global', resourceId, resourceName, pluginId });
  };

  const handleUninstallResource = async (linkIds: string[], pluginId: string) => {
    await useRegistryStore.getState().uninstallResource(linkIds, pluginId);
  };

  const filteredResources = getFilteredResources();

  const title = selectedSource === 'local'
    ? t('library.title')
    : registries.find((registry) => registry.id === selectedSource)?.name ?? t('library.selectedRegistry');

  const description = selectedSource === 'local'
    ? t('library.subtitle')
    : t('library.remoteSubtitle');

  return (
    <PageShell className="gap-5">
      <PageHeader
        eyebrow={t('nav.library')}
        title={title}
        description={description}
        actions={
          selectedSource === 'local' ? (
            <>
              <LinkHealthBadge linkHealth={linkHealth} onCheck={checkLinkHealth} loading={loading} />
              {pageTab === 'resources' ? (
                <Button size="sm" onClick={() => setCreateOpen(true)}>
                  <Plus className="mr-1 size-4" />
                  {t('common.create')}
                </Button>
              ) : null}
            </>
          ) : null
        }
      />

      <div className="flex min-h-0 gap-5">
        <LibrarySourcePanel />
        <div className="min-w-0 flex-1 space-y-5">
          {selectedSource === 'local' ? (
            <>
              <ToolbarRow>
                <Tabs value={pageTab} onValueChange={(value) => setPageTab(value as 'resources' | 'plugin-packs')}>
                  <TabsList className="rounded-md">
                    <TabsTrigger value="resources">{t('library.resources')}</TabsTrigger>
                    <TabsTrigger value="plugin-packs">{t('library.pluginPacks')}</TabsTrigger>
                  </TabsList>

                  <TabsContent value="resources" className="mt-4 space-y-4">
                    <div className="flex items-center gap-2">
                      <ResourceTypeTabs activeTab={activeTab} onTabChange={(tab) => setActiveTab(tab as import('@/types/v2').ResourceType)} />
                      <Button
                        variant={installFilter === 'installed' ? 'default' : 'outline'}
                        size="sm"
                        className="shrink-0 rounded-md text-xs"
                        onClick={() => setInstallFilter(installFilter === 'installed' ? 'all' : 'installed')}
                      >
                        {installFilter === 'installed' ? t('library.showAll') : t('library.showInstalled')}
                      </Button>
                    </div>
                  </TabsContent>
                  <TabsContent value="plugin-packs" className="hidden" />
                </Tabs>
              </ToolbarRow>

              {error ? <InlineStatus tone="danger">{error}</InlineStatus> : null}

              <Tabs value={pageTab} onValueChange={(value) => setPageTab(value as 'resources' | 'plugin-packs')}>
                <TabsContent value="resources" className="mt-0">
                  <PanelSection>
                    {loading ? (
                      <div className="flex items-center justify-center py-16 text-muted-foreground">
                        {t('common.loading')}
                      </div>
                    ) : filteredResources.length === 0 ? (
                      <EmptyState title={t('library.noResources')} />
                    ) : (
                      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
                        {filteredResources.map((item) => {
                          const status = item.installations.length === 0 ? 'not_installed'
                            : item.resource.is_draft === -1 ? 'removed'
                            : item.has_update ? 'update_available'
                            : 'up_to_date';
                          return (
                          <ResourceCard
                            key={item.resource.id}
                            resource={item.resource}
                            status={status}
                            links={[]}
                            projectNames={projectNames}
                            onClick={() => navigateToResource(navigate, item.resource)}
                            onInstallToProject={() => {
                              setSelectedResourceId(item.resource.id);
                              setSelectedResourceName(item.resource.name);
                              setInstallDialogOpen(true);
                            }}
                            onInstallToGlobal={() => {
                              setSelectedResourceId(item.resource.id);
                              setSelectedResourceName(item.resource.name);
                              setDeployDialogOpen(true);
                            }}
                            onUpdate={item.has_update ? async () => {
                              await apiUpdateInstalledResource(item.resource.id);
                              loadResources();
                            } : undefined}
                            onRetain={item.resource.is_draft === -1 ? async () => {
                              await apiRetainAsLibrary(item.resource.id);
                              loadResources();
                            } : undefined}
                            onUninstall={undefined}
                            extraActions={
                              <>
                                <Button
                                  variant="ghost"
                                  size="icon-sm"
                                  className="text-muted-foreground hover:text-primary"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    setAddToPluginResourceId(item.resource.id);
                                  }}
                                  title={t('library.addToPluginPack')}
                                >
                                  <Package className="size-3.5" />
                                </Button>
                                <Button
                                  variant="ghost"
                                  size="icon-sm"
                                  className="text-muted-foreground hover:text-destructive"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    setPendingDelete(item.resource);
                                  }}
                                  title={t('common.delete')}
                                >
                                  <Trash2 className="size-3.5" />
                                </Button>
                              </>
                            }
                          />
                        );
                        })}
                      </div>
                    )}
                  </PanelSection>
                </TabsContent>

                <TabsContent value="plugin-packs" className="mt-0">
                  <PanelSection>
                    <LibraryPluginTab />
                  </PanelSection>
                </TabsContent>
              </Tabs>
            </>
          ) : (
            <PanelSection title={title} description={description}>
              <RegistryPluginList
                registryId={selectedSource}
                onInstallPlugin={(pluginId, pluginName) => {
                  setProjectInstallPluginId(pluginId);
                  setProjectInstallPluginName(pluginName);
                  setProjectInstallDialogOpen(true);
                }}
                onInstallPluginToGlobal={(pluginId, pluginName) => {
                  setGlobalInstallPluginId(pluginId);
                  setGlobalInstallPluginName(pluginName);
                  setGlobalInstallDialogOpen(true);
                }}
                onInstallResource={handleInstallResource}
                onInstallResourceToGlobal={handleInstallResourceToGlobal}
                onUninstallResource={handleUninstallResource}
                projectNames={projectNames}
              />
            </PanelSection>
          )}
        </div>
      </div>

      <InstallToProjectDialog
        open={installDialogOpen}
        onOpenChange={setInstallDialogOpen}
        resourceName={selectedResourceName}
        onInstall={async (projectId, linkType: LinkType) => {
          if (!selectedResourceId) return;
          await installToProject(selectedResourceId, projectId, linkType);
          loadResources();
        }}
      />

      <DeployToGlobalDialog
        open={deployDialogOpen}
        onOpenChange={setDeployDialogOpen}
        resourceName={selectedResourceName}
        onDeploy={async (linkType: LinkType) => {
          if (!selectedResourceId) return;
          await deployToGlobal(selectedResourceId, linkType);
          loadResources();
        }}
      />

      {addToPluginResourceId ? (
        <AddToPluginDialog
          open={!!addToPluginResourceId}
          onOpenChange={(open) => { if (!open) setAddToPluginResourceId(null); }}
          resourceId={addToPluginResourceId}
        />
      ) : null}

      <InstallPluginToProjectDialog
        open={projectInstallDialogOpen}
        onOpenChange={setProjectInstallDialogOpen}
        pluginName={projectInstallPluginName}
        onConfirm={async (projectId) => {
          if (projectInstallPluginId) {
            await installPluginToProject(projectInstallPluginId, projectId);
          }
        }}
      />

      <InstallPluginToGlobalDialog
        open={globalInstallDialogOpen}
        onOpenChange={setGlobalInstallDialogOpen}
        pluginName={globalInstallPluginName}
        onConfirm={async () => {
          if (globalInstallPluginId) {
            await installPluginToGlobal(globalInstallPluginId);
          }
        }}
      />

      <InstallPluginToProjectDialog
        open={resourceInstall?.mode === 'project'}
        onOpenChange={(open) => { if (!open) setResourceInstall(null); }}
        pluginName=""
        resourceName={resourceInstall?.resourceName ?? ''}
        onConfirm={async (projectId) => {
          if (resourceInstall) {
            await useRegistryStore.getState().installResourceToProject(resourceInstall.resourceId, projectId, resourceInstall.pluginId);
          }
        }}
      />

      <InstallPluginToGlobalDialog
        open={resourceInstall?.mode === 'global'}
        onOpenChange={(open) => { if (!open) setResourceInstall(null); }}
        pluginName=""
        resourceName={resourceInstall?.resourceName ?? ''}
        onConfirm={async () => {
          if (resourceInstall) {
            await useRegistryStore.getState().installResourceToGlobal(resourceInstall.resourceId, resourceInstall.pluginId);
          }
        }}
      />

      <CreateResourceDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        resourceType={activeTab}
        onSubmit={async (type, name, content) => {
          const resource = await createResource(type, name, '', content);
          return resource;
        }}
        onCreated={(resource) => navigateToResource(navigate, resource)}
      />

      <DeleteConfirmDialog
        open={!!pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={(deleteFromDisk) => {
          if (pendingDelete) {
            deleteResource(pendingDelete.id, deleteFromDisk);
            setPendingDelete(null);
          }
        }}
        title={t('library.deleteTitle')}
        name={pendingDelete?.name ?? ''}
        path={pendingDelete?.source_path ?? ''}
      />
    </PageShell>
  );
}
