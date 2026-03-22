import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { ChevronDown, ChevronRight, Download, ExternalLink, Globe, Server } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useRegistryStore } from '@/stores/registry-store';
import { ResourceTable } from '@/components/shared/ResourceTable';
import { McpServerList } from '@/components/shared/McpServerList';
import type { RegistryPlugin } from '@/types/v2';

interface RegistryPluginItemProps {
  plugin: RegistryPlugin;
  onInstall: (pluginId: string, pluginName: string) => void;
  onInstallToGlobal: (pluginId: string, pluginName: string) => void;
}

export function RegistryPluginItem({ plugin, onInstall, onInstallToGlobal }: RegistryPluginItemProps) {
  const navigate = useNavigate();
  const { expandedPlugins, togglePluginExpanded, pluginResources, loadPluginResources, pluginMcpServers, loadPluginMcpServers } =
    useRegistryStore();

  const isExpanded = expandedPlugins.has(plugin.id);
  const resources = pluginResources[plugin.id] || [];
  const mcpServers = pluginMcpServers[plugin.id] || [];

  useEffect(() => {
    if (isExpanded && !pluginResources[plugin.id]) {
      loadPluginResources(plugin.id);
    }
    if (isExpanded && !pluginMcpServers[plugin.id]) {
      loadPluginMcpServers(plugin.id);
    }
  }, [isExpanded, plugin.id, pluginResources, loadPluginResources, pluginMcpServers, loadPluginMcpServers]);

  return (
    <div className="border rounded-lg">
      <div
        className="flex items-center justify-between p-3 cursor-pointer hover:bg-muted/50"
        onClick={() => togglePluginExpanded(plugin.id)}
      >
        <div className="flex items-center gap-2">
          {isExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
          <span
            className="font-medium hover:underline"
            onClick={(e) => {
              e.stopPropagation();
              navigate(`/registry-plugins/${plugin.id}`);
            }}
          >
            {plugin.name}
          </span>
          {plugin.category && (
            <Badge variant="secondary">{plugin.category}</Badge>
          )}
          {plugin.source_type === 'external' && (
            <Badge variant="outline">external</Badge>
          )}
        </div>
        <div className="flex items-center gap-2">
          {plugin.homepage && (
            <Button
              variant="ghost"
              size="icon"
              onClick={(e) => {
                e.stopPropagation();
                window.open(plugin.homepage!, '_blank');
              }}
            >
              <ExternalLink className="h-4 w-4" />
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={(e) => {
              e.stopPropagation();
              onInstall(plugin.id, plugin.name);
            }}
          >
            <Download className="mr-1 h-4 w-4" />
            安装到项目
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={(e) => {
              e.stopPropagation();
              onInstallToGlobal(plugin.id, plugin.name);
            }}
          >
            <Globe className="mr-1 h-4 w-4" />
            安装到全局
          </Button>
        </div>
      </div>
      {isExpanded && (
        <div className="border-t px-4 py-3 space-y-2">
          {plugin.description && (
            <p className="text-sm text-muted-foreground">{plugin.description}</p>
          )}
          <ResourceTable resources={resources} />
          {mcpServers.length > 0 && (
            <>
              <div className="flex items-center gap-2 pt-2">
                <Server className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium">MCP Servers</span>
              </div>
              <McpServerList servers={mcpServers} />
            </>
          )}
        </div>
      )}
    </div>
  );
}
