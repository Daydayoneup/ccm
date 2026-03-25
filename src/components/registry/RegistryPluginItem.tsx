import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { ChevronDown, ChevronRight, Download, ExternalLink, Globe, Server } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useRegistryStore } from '@/stores/registry-store';
import { McpServerList } from '@/components/shared/McpServerList';
import { ResourceInstallDropdown } from './ResourceInstallDropdown';
import { ResourceUninstallPopover } from './ResourceUninstallPopover';
import type { RegistryPlugin, ResourceLink } from '@/types/v2';

interface RegistryPluginItemProps {
  plugin: RegistryPlugin;
  onInstall: (pluginId: string, pluginName: string) => void;
  onInstallToGlobal: (pluginId: string, pluginName: string) => void;
  onInstallResource: (resourceId: string, resourceName: string, pluginId: string) => void;
  onInstallResourceToGlobal: (resourceId: string, resourceName: string, pluginId: string) => void;
  onUninstallResource: (linkIds: string[], pluginId: string) => void;
  projectNames: Record<string, string>;
}

export function RegistryPluginItem({ plugin, onInstall, onInstallToGlobal, onInstallResource, onInstallResourceToGlobal, onUninstallResource, projectNames }: RegistryPluginItemProps) {
  const navigate = useNavigate();
  const { expandedPlugins, togglePluginExpanded, pluginResources, loadPluginResources, pluginMcpServers, loadPluginMcpServers, resourceInstallStatus, loadResourceInstallStatus } =
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
    if (isExpanded) {
      loadResourceInstallStatus(plugin.id);
    }
  }, [isExpanded, plugin.id, pluginResources, loadPluginResources, pluginMcpServers, loadPluginMcpServers, loadResourceInstallStatus]);

  return (
    <div className="rounded-md border">
      <div
        className="flex cursor-pointer flex-col gap-3 p-3 hover:bg-muted/50 lg:flex-row lg:items-center lg:justify-between"
        onClick={() => togglePluginExpanded(plugin.id)}
      >
        <div className="flex min-w-0 flex-1 items-start gap-2">
          <div className="pt-1">
            {isExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 flex-wrap items-start gap-2">
              <span
                className="min-w-0 break-words font-medium hover:underline"
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
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 lg:ml-4 lg:flex-nowrap">
          {plugin.homepage && (
            <Button
              variant="ghost"
              size="icon"
              className="shrink-0"
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
            className="min-w-0 shrink-0"
            onClick={(e) => {
              e.stopPropagation();
              onInstall(plugin.id, plugin.name);
            }}
          >
            <Download className="mr-1 h-4 w-4" />
            全部安装到项目
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="min-w-0 shrink-0"
            onClick={(e) => {
              e.stopPropagation();
              onInstallToGlobal(plugin.id, plugin.name);
            }}
          >
            <Globe className="mr-1 h-4 w-4" />
            全部安装到全局
          </Button>
        </div>
      </div>
      {isExpanded && (
        <div className="space-y-2 border-t px-4 py-3">
          {plugin.description && (
            <p className="text-sm text-muted-foreground">{plugin.description}</p>
          )}
          {resources.length > 0 && (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
              {resources.map((resource) => {
                const installStatus = resourceInstallStatus[plugin.id] || {};
                const resourceLinks: ResourceLink[] = installStatus[resource.id] || [];
                const isInstalled = resourceLinks.length > 0;

                return (
                  <div
                    key={resource.id}
                    className="rounded-md border bg-card/90 p-4"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="truncate text-sm font-medium">{resource.name}</span>
                          {isInstalled && (
                            <Badge variant="secondary" className="shrink-0 text-xs">已安装</Badge>
                          )}
                        </div>
                        <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">
                          {resource.description || '暂无描述'}
                        </p>
                      </div>
                      <div className="shrink-0">
                        {isInstalled ? (
                          <ResourceUninstallPopover
                            links={resourceLinks}
                            projectNames={projectNames}
                            onUninstall={(linkIds) => onUninstallResource(linkIds, plugin.id)}
                          />
                        ) : (
                          <ResourceInstallDropdown
                            onInstallToProject={() => onInstallResource(resource.id, resource.name, plugin.id)}
                            onInstallToGlobal={() => onInstallResourceToGlobal(resource.id, resource.name, plugin.id)}
                          />
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
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
