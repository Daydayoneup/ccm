import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useLibraryStore } from '@/stores/library-store-v2';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { CreateLibraryResourceDialog } from '@/components/library/CreateLibraryResourceDialog';
import { InstallToProjectDialog } from '@/components/library/InstallToProjectDialog';
import { DeployToGlobalDialog } from '@/components/library/DeployToGlobalDialog';
import { LinkHealthBadge } from '@/components/library/LinkHealthBadge';
import { LibraryPluginTab } from '@/components/library/LibraryPluginTab';
import { AddToPluginDialog } from '@/components/library/AddToPluginDialog';
import { LibrarySourcePanel } from '@/components/library/LibrarySourcePanel';
import { InstallPluginToGlobalDialog } from '@/components/library/InstallPluginToGlobalDialog';
import { InstallPluginToProjectDialog } from '@/components/library/InstallPluginToProjectDialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Trash2, Download, Globe, Package } from 'lucide-react';
import { DeleteConfirmDialog } from '@/components/shared/DeleteConfirmDialog';
import type { Resource, LinkType } from '@/types/v2';
import { useRegistryStore } from '@/stores/registry-store';
import { RegistryPluginList } from '@/components/registry/RegistryPluginList';

export function LibraryPage_v2() {
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

  const navigate = useNavigate();

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

  const handleInstallClick = (resourceId: string, resourceName: string) => {
    setSelectedResourceId(resourceId);
    setSelectedResourceName(resourceName);
    setInstallDialogOpen(true);
  };

  const handleDeployClick = (resourceId: string, resourceName: string) => {
    setSelectedResourceId(resourceId);
    setSelectedResourceName(resourceName);
    setDeployDialogOpen(true);
  };

  const handleInstall = async (projectId: string, linkType: LinkType) => {
    if (!selectedResourceId) return;
    await installToProject(selectedResourceId, projectId, linkType);
  };

  const handleDeploy = async (linkType: LinkType) => {
    if (!selectedResourceId) return;
    await deployToGlobal(selectedResourceId, linkType);
  };

  const handleInstallPluginToProject = (pluginId: string, pluginName: string) => {
    setProjectInstallPluginId(pluginId);
    setProjectInstallPluginName(pluginName);
    setProjectInstallDialogOpen(true);
  };

  const handleInstallPluginToGlobal = (pluginId: string, pluginName: string) => {
    setGlobalInstallPluginId(pluginId);
    setGlobalInstallPluginName(pluginName);
    setGlobalInstallDialogOpen(true);
  };

  const borderClasses: Record<string, string> = {
    skill: 'res-border-skill',
    agent: 'res-border-agent',
    rule: 'res-border-rule',
    hook: 'res-border-hook',
    mcp_server: 'res-border-mcp_server',
    command: 'res-border-command',
  };

  return (
    <div className="flex h-full gap-6 p-8">
      <LibrarySourcePanel />

      <div className="flex-1 space-y-6 min-w-0">
        {selectedSource === 'local' ? (
          <>
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-2xl font-bold tracking-tight">Resource Library</h1>
                <p className="mt-1 text-sm text-muted-foreground">
                  Central resource library — install to projects or deploy globally
                </p>
              </div>
              <div className="flex items-center gap-3">
                <LinkHealthBadge
                  linkHealth={linkHealth}
                  onCheck={checkLinkHealth}
                  loading={loading}
                />
                {pageTab === 'resources' && (
                  <CreateLibraryResourceDialog
                    onSubmit={createResource}
                    defaultType={activeTab}
                  />
                )}
              </div>
            </div>

            <Tabs value={pageTab} onValueChange={(v) => setPageTab(v as 'resources' | 'plugin-packs')}>
              <TabsList>
                <TabsTrigger value="resources">资源</TabsTrigger>
                <TabsTrigger value="plugin-packs">插件包</TabsTrigger>
              </TabsList>

              <TabsContent value="resources" className="space-y-4">
                <ResourceTypeTabs activeTab={activeTab} onTabChange={(tab) => setActiveTab(tab as import('@/types/v2').ResourceType)} />

                {error && (
                  <div className="rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                    {error}
                  </div>
                )}

                {loading ? (
                  <div className="flex items-center justify-center py-16 text-muted-foreground">
                    Loading...
                  </div>
                ) : filteredResources.length === 0 ? (
                  <div className="flex items-center justify-center rounded-xl border border-dashed py-16 text-muted-foreground">
                    No resources found
                  </div>
                ) : (
                  <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
                    {filteredResources.map((resource) => (
                      <div
                        key={resource.id}
                        className={`card-glow group cursor-pointer rounded-xl border bg-card p-4 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/5 ${borderClasses[resource.resource_type] || ''}`}
                        onClick={() => navigate(`/editor?file=${encodeURIComponent(resource.source_path)}`)}
                      >
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="truncate font-semibold text-sm">{resource.name}</span>
                            {resource.description && (
                              <Badge variant="outline" className="shrink-0 text-[10px] font-normal">
                                {resource.description}
                              </Badge>
                            )}
                          </div>
                          <p className="mt-1.5 truncate font-mono text-xs text-muted-foreground">
                            {resource.source_path}
                          </p>
                        </div>
                        <div className="mt-3 flex items-center justify-end gap-0.5 border-t border-border/50 pt-3">
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="text-muted-foreground hover:text-primary"
                            onClick={(e) => { e.stopPropagation(); handleInstallClick(resource.id, resource.name); }}
                            title="Install to Project"
                          >
                            <Download className="size-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="text-muted-foreground hover:text-primary"
                            onClick={(e) => { e.stopPropagation(); handleDeployClick(resource.id, resource.name); }}
                            title="Deploy to Global"
                          >
                            <Globe className="size-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="text-muted-foreground hover:text-primary"
                            onClick={(e) => { e.stopPropagation(); setAddToPluginResourceId(resource.id); }}
                            title="添加到插件包"
                          >
                            <Package className="size-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="text-muted-foreground hover:text-destructive"
                            onClick={(e) => { e.stopPropagation(); setPendingDelete(resource); }}
                            title="Delete"
                          >
                            <Trash2 className="size-3.5" />
                          </Button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </TabsContent>

              <TabsContent value="plugin-packs">
                <LibraryPluginTab />
              </TabsContent>
            </Tabs>

            <InstallToProjectDialog
              open={installDialogOpen}
              onOpenChange={setInstallDialogOpen}
              resourceName={selectedResourceName}
              onInstall={handleInstall}
            />

            <DeployToGlobalDialog
              open={deployDialogOpen}
              onOpenChange={setDeployDialogOpen}
              resourceName={selectedResourceName}
              onDeploy={handleDeploy}
            />

            {addToPluginResourceId && (
              <AddToPluginDialog
                open={!!addToPluginResourceId}
                onOpenChange={(open) => !open && setAddToPluginResourceId(null)}
                resourceId={addToPluginResourceId}
              />
            )}
          </>
        ) : (
          <>
            <div className="flex items-center justify-between">
              <h1 className="text-2xl font-bold tracking-tight">
                {registries.find(r => r.id === selectedSource)?.name ?? '仓库'}
              </h1>
            </div>
            <RegistryPluginList
              registryId={selectedSource}
              onInstallPlugin={handleInstallPluginToProject}
              onInstallPluginToGlobal={handleInstallPluginToGlobal}
            />
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
          </>
        )}
      </div>
      <DeleteConfirmDialog
        open={!!pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={(deleteFromDisk) => {
          if (pendingDelete) {
            deleteResource(pendingDelete.id, deleteFromDisk);
            setPendingDelete(null);
          }
        }}
        title="确认删除资源"
        name={pendingDelete?.name ?? ''}
        path={pendingDelete?.source_path ?? ''}
      />
    </div>
  );
}
