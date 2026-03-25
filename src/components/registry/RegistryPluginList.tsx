import { useEffect, useState } from 'react';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useRegistryStore } from '@/stores/registry-store';
import { RegistryPluginItem } from './RegistryPluginItem';

interface RegistryPluginListProps {
  registryId: string;
  onInstallPlugin?: (pluginId: string, pluginName: string) => void;
  onInstallPluginToGlobal?: (pluginId: string, pluginName: string) => void;
  onInstallResource?: (resourceId: string, resourceName: string, pluginId: string) => void;
  onInstallResourceToGlobal?: (resourceId: string, resourceName: string, pluginId: string) => void;
  onUninstallResource?: (linkIds: string[], pluginId: string) => void;
  projectNames?: Record<string, string>;
}

export function RegistryPluginList({ registryId, onInstallPlugin, onInstallPluginToGlobal, onInstallResource, onInstallResourceToGlobal, onUninstallResource, projectNames }: RegistryPluginListProps) {
  const { registryPlugins, loadRegistryPlugins } = useRegistryStore();
  const [search, setSearch] = useState('');
  const [categoryFilter, setCategoryFilter] = useState<string>('all');

  useEffect(() => {
    loadRegistryPlugins(registryId);
  }, [registryId, loadRegistryPlugins]);

  const categories = [...new Set(registryPlugins.map((p) => p.category).filter(Boolean))] as string[];

  const filteredPlugins = registryPlugins.filter((p) => {
    const matchesSearch = !search || p.name.toLowerCase().includes(search.toLowerCase());
    const matchesCategory = categoryFilter === 'all' || p.category === categoryFilter;
    return matchesSearch && matchesCategory;
  });

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-2 sm:flex-row">
        <Input
          placeholder="搜索插件..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full sm:max-w-sm"
        />
        {categories.length > 0 && (
          <Select value={categoryFilter} onValueChange={setCategoryFilter}>
            <SelectTrigger className="w-full sm:w-40">
              <SelectValue placeholder="所有分类" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">所有分类</SelectItem>
              {categories.map((cat) => (
                <SelectItem key={cat} value={cat}>
                  {cat}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      <div className="space-y-2">
        {filteredPlugins.map((plugin) => (
          <RegistryPluginItem
            key={plugin.id}
            plugin={plugin}
            onInstall={(id, name) => onInstallPlugin?.(id, name)}
            onInstallToGlobal={(id, name) => onInstallPluginToGlobal?.(id, name)}
            onInstallResource={onInstallResource ?? (() => {})}
            onInstallResourceToGlobal={onInstallResourceToGlobal ?? (() => {})}
            onUninstallResource={onUninstallResource ?? (() => {})}
            projectNames={projectNames ?? {}}
          />
        ))}
        {filteredPlugins.length === 0 && (
          <p className="text-center text-muted-foreground py-8">没有找到插件</p>
        )}
      </div>
    </div>
  );
}
