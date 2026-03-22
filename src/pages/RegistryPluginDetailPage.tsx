import { useEffect } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useRegistryStore } from '@/stores/registry-store';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { ResourceTable } from '@/components/shared/ResourceTable';
import { McpServerList } from '@/components/shared/McpServerList';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ArrowLeft } from 'lucide-react';
import type { ResourceType, McpServer } from '@/types/v2';

type TabValue = ResourceType | 'mcp';

export function RegistryPluginDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { registryPlugins, pluginResources, loadPluginResources, pluginMcpServers, loadPluginMcpServers } = useRegistryStore();

  const plugin = registryPlugins.find((p) => p.id === id);
  const resources = id ? pluginResources[id] || [] : [];
  const mcpServers: McpServer[] = id ? pluginMcpServers[id] || [] : [];

  useEffect(() => {
    if (id && !pluginResources[id]) {
      loadPluginResources(id);
    }
    if (id && !pluginMcpServers[id]) {
      loadPluginMcpServers(id);
    }
  }, [id, pluginResources, loadPluginResources, pluginMcpServers, loadPluginMcpServers]);

  const activeTab = useRegistryStore((s) => s.activeTab);
  const setActiveTab = useRegistryStore((s) => s.setActiveTab);

  const filteredResources = activeTab === 'mcp' ? [] : resources.filter((r) => r.resource_type === activeTab);

  if (!plugin) {
    return (
      <div className="flex items-center justify-center p-6">
        <p className="text-muted-foreground">Plugin not found</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ArrowLeft className="size-4" />
        </Button>
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold">{plugin.name}</h1>
            {plugin.category && (
              <Badge variant="secondary">{plugin.category}</Badge>
            )}
            <Badge variant={plugin.source_type === 'external' ? 'outline' : 'default'}>
              {plugin.source_type}
            </Badge>
          </div>
          <p className="text-sm text-muted-foreground">{plugin.source_path}</p>
        </div>
      </div>

      <ResourceTypeTabs activeTab={activeTab} onTabChange={(v) => setActiveTab(v as TabValue)} includeMcp />

      {activeTab === 'mcp' ? (
        mcpServers.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-muted-foreground">
            No MCP servers found
          </div>
        ) : (
          <McpServerList servers={mcpServers} />
        )
      ) : filteredResources.length === 0 ? (
        <div className="flex items-center justify-center py-12 text-muted-foreground">
          No resources found
        </div>
      ) : (
        <ResourceTable resources={filteredResources} />
      )}
    </div>
  );
}
