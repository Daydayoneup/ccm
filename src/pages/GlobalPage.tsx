import { useEffect } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { useGlobalStore } from '@/stores/global-store';
import { usePluginStore } from '@/stores/plugin-store';
import type { ResourceType } from '@/types/v2';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { ResourceTable } from '@/components/shared/ResourceTable';
import { McpServerList } from '@/components/shared/McpServerList';
import { CreateGlobalResourceDialog } from '@/components/global/CreateGlobalResourceDialog';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Package } from 'lucide-react';

export function GlobalPage() {
  const {
    loading,
    error,
    activeTab,
    globalMcpServers,
    loadResources,
    loadGlobalMcpServers,
    createResource,
    deleteResource,
    backupToLibrary,
    setActiveTab,
    getFilteredResources,
  } = useGlobalStore();

  const {
    plugins,
    loading: pluginLoading,
    scanning: pluginScanning,
    error: pluginError,
    loadPlugins,
    scanPlugins,
  } = usePluginStore();

  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  useEffect(() => {
    const tab = searchParams.get('tab');
    const validTabs = ['skill', 'agent', 'rule', 'hook', 'command', 'mcp', 'plugin'] as const;
    if (tab && (validTabs as readonly string[]).includes(tab)) {
      setActiveTab(tab as typeof validTabs[number]);
    }
  }, [searchParams, setActiveTab]);

  useEffect(() => {
    loadResources();
    loadGlobalMcpServers();
  }, [loadResources, loadGlobalMcpServers]);

  useEffect(() => {
    if (activeTab === 'plugin') {
      loadPlugins();
    }
  }, [activeTab, loadPlugins]);

  const filteredResources = getFilteredResources();

  return (
    <div className="space-y-6 p-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Global Resources</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Resources in ~/.claude/ — active across all projects
          </p>
        </div>
        {activeTab === 'plugin' ? (
          <Button size="sm" className="rounded-lg" onClick={scanPlugins} disabled={pluginScanning}>
            <RefreshCw className={`mr-1.5 size-3.5 ${pluginScanning ? 'animate-spin' : ''}`} />
            {pluginScanning ? 'Scanning...' : 'Scan'}
          </Button>
        ) : activeTab !== 'mcp' && (
          <CreateGlobalResourceDialog
            onSubmit={createResource}
            defaultType={activeTab as ResourceType}
          />
        )}
      </div>

      <ResourceTypeTabs activeTab={activeTab} onTabChange={(v) => setActiveTab(v as typeof activeTab)} includeMcp includePlugin />

      {(error || pluginError) && (
        <div className="rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          {error ?? pluginError}
        </div>
      )}

      {activeTab === 'mcp' ? (
        <McpServerList
          servers={globalMcpServers}
          emptyMessage="No global MCP servers found."
          emptyHint="Global MCP servers come from ~/.claude/.mcp.json and enabled plugins."
        />
      ) : activeTab === 'plugin' ? (
        pluginLoading ? (
          <div className="flex items-center justify-center py-16 text-muted-foreground">
            Loading...
          </div>
        ) : plugins.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-xl border border-dashed py-16 text-muted-foreground">
            <Package className="mb-4 size-12 opacity-40" />
            <p>No plugins found.</p>
            <p className="text-sm">Click Scan to discover installed plugins.</p>
          </div>
        ) : (
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {plugins.map((plugin) => (
              <Card
                key={plugin.id}
                className="card-glow cursor-pointer transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/5"
                onClick={() => navigate(`/plugins/${plugin.id}`)}
              >
                <CardHeader className="pb-2">
                  <CardTitle className="text-base">{plugin.name}</CardTitle>
                  <CardDescription className="truncate font-mono text-xs">
                    {plugin.install_path ?? 'Unknown path'}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap items-center gap-2">
                    {plugin.version && (
                      <Badge variant="secondary">v{plugin.version}</Badge>
                    )}
                    {plugin.scope && (
                      <Badge variant="outline">{plugin.scope}</Badge>
                    )}
                    <Badge variant={plugin.status === 'installed' ? 'default' : 'secondary'}>
                      {plugin.status}
                    </Badge>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )
      ) : loading ? (
        <div className="flex items-center justify-center py-16 text-muted-foreground">
          Loading...
        </div>
      ) : (
        <ResourceTable
          resources={filteredResources}
          onDelete={deleteResource}
          onBackup={backupToLibrary}
        />
      )}
    </div>
  );
}
