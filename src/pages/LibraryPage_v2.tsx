import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Download, Globe, Package, Trash2 } from 'lucide-react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { DeleteConfirmDialog } from '@/components/shared/DeleteConfirmDialog';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { CreateLibraryResourceDialog } from '@/components/library/CreateLibraryResourceDialog';
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
import type { LinkType, Resource } from '@/types/v2';
import { RegistryPluginList } from '@/components/registry/RegistryPluginList';

const borderClasses: Record<string, string> = {
  skill: 'res-border-skill',
  agent: 'res-border-agent',
  rule: 'res-border-rule',
  hook: 'res-border-hook',
  mcp_server: 'res-border-mcp_server',
  command: 'res-border-command',
};

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
  } = useLibraryStore();
  const { registries, installPluginToProject, installPluginToGlobal } = useRegistryStore();

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

  useEffect(() => {
    loadResources();
  }, [loadResources]);

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
                <CreateLibraryResourceDialog
                onSubmit={createResource}
                defaultType={activeTab}
                onCreated={(resource) => {
                  const filePath = resource.source_path;
                  const extra = resource.resource_type === 'skill'
                    ? `&resource_id=${resource.id}&type=skill&scope=library`
                    : '';
                  navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
                }}
              />
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
                    <ResourceTypeTabs activeTab={activeTab} onTabChange={(tab) => setActiveTab(tab as import('@/types/v2').ResourceType)} />
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
                        {filteredResources.map((resource) => (
                          <div
                            key={resource.id}
                            className={`card-glow group cursor-pointer rounded-md border bg-card/90 p-5 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-[0_20px_50px_rgba(15,23,42,0.10)] ${borderClasses[resource.resource_type] || ''}`}
                            onClick={() => {
                              const filePath = resource.source_path;
                              const extra = resource.resource_type === 'skill'
                                ? `&resource_id=${resource.id}&type=skill&scope=${selectedSource === 'local' ? 'library' : 'registry'}`
                                : '';
                              navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
                            }}
                          >
                            <div className="min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="truncate text-sm font-semibold">{resource.name}</span>
                                {resource.description ? (
                                  <Badge variant="outline" className="shrink-0 text-[10px] font-normal">
                                    {resource.description}
                                  </Badge>
                                ) : null}
                              </div>
                              <p className="mt-3 truncate text-xs font-mono text-muted-foreground">{resource.source_path}</p>
                            </div>
                            <div className="mt-4 flex items-center justify-end gap-1 border-t border-border/50 pt-3">
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                className="text-muted-foreground hover:text-primary"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setSelectedResourceId(resource.id);
                                  setSelectedResourceName(resource.name);
                                  setInstallDialogOpen(true);
                                }}
                                title={t('library.installToProject')}
                              >
                                <Download className="size-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                className="text-muted-foreground hover:text-primary"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setSelectedResourceId(resource.id);
                                  setSelectedResourceName(resource.name);
                                  setDeployDialogOpen(true);
                                }}
                                title={t('library.deployToGlobal')}
                              >
                                <Globe className="size-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                className="text-muted-foreground hover:text-primary"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setAddToPluginResourceId(resource.id);
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
                                  setPendingDelete(resource);
                                }}
                                title={t('common.delete')}
                              >
                                <Trash2 className="size-3.5" />
                              </Button>
                            </div>
                          </div>
                        ))}
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
        }}
      />

      <DeployToGlobalDialog
        open={deployDialogOpen}
        onOpenChange={setDeployDialogOpen}
        resourceName={selectedResourceName}
        onDeploy={async (linkType: LinkType) => {
          if (!selectedResourceId) return;
          await deployToGlobal(selectedResourceId, linkType);
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
