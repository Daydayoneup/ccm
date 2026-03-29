import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { navigateToResource } from '@/lib/navigation';
import { ChevronDown, ChevronRight, Download, ExternalLink, Globe } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useRegistryStore } from '@/stores/registry-store';
import { ResourceCard, getResourceInstallStatus } from '@/components/shared/ResourceCard';
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

export function RegistryPluginItem({ plugin, onInstall, onInstallToGlobal, onInstallResource, onInstallResourceToGlobal }: RegistryPluginItemProps) {
  const navigate = useNavigate();
  const { expandedPlugins, togglePluginExpanded, pluginResources, loadPluginResources,
    resourceInstallStatus, loadResourceInstallStatus } = useRegistryStore();

  const isExpanded = expandedPlugins.has(plugin.id);
  const resources = pluginResources[plugin.id] || [];

  useEffect(() => {
    if (isExpanded && !pluginResources[plugin.id]) {
      loadPluginResources(plugin.id);
    }
    if (isExpanded) {
      loadResourceInstallStatus(plugin.id);
    }
  }, [isExpanded, plugin.id, pluginResources, loadPluginResources, loadResourceInstallStatus]);

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
              <span className="min-w-0 break-words font-medium">
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
                const installStatusMap = resourceInstallStatus[plugin.id] || {};
                const resourceLinks: ResourceLink[] = installStatusMap[resource.id] || [];
                const status = getResourceInstallStatus(resource, resourceLinks);

                return (
                  <ResourceCard
                    key={resource.id}
                    resource={resource}
                    status={status}
                    onInstallToProject={() => onInstallResource(resource.id, resource.name, plugin.id)}
                    onInstallToGlobal={() => onInstallResourceToGlobal(resource.id, resource.name, plugin.id)}
                    onClick={() => navigateToResource(navigate, resource)}
                  />
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
