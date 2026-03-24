import { useEffect } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { usePluginStore } from '@/stores/plugin-store';
import { ResourceTypeTabs } from '@/components/shared/ResourceTypeTabs';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ArrowLeft, Pencil, Archive } from 'lucide-react';

export function PluginDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const {
    plugins,
    loading,
    error,
    activeTab,
    selectPlugin,
    loadPlugins,
    loadPluginResources,
    extractToLibrary,
    setActiveTab,
    getFilteredResources,
  } = usePluginStore();

  const plugin = plugins.find((p) => p.id === id);

  useEffect(() => {
    if (plugins.length === 0) {
      loadPlugins();
    }
  }, [plugins.length, loadPlugins]);

  useEffect(() => {
    if (plugin) {
      selectPlugin(plugin);
    }
  }, [plugin?.id, selectPlugin]);

  useEffect(() => {
    if (id) {
      loadPluginResources(id);
    }
  }, [id, loadPluginResources]);

  const filteredResources = getFilteredResources();

  const handleExtract = async (resourceId: string) => {
    try {
      await extractToLibrary(resourceId);
    } catch (e) {
      console.error('Failed to extract to library:', e);
    }
  };

  if (!plugin && plugins.length > 0) {
    return (
      <div className="flex items-center justify-center p-6">
        <p className="text-muted-foreground">Plugin not found</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={() => navigate('/plugins')}>
          <ArrowLeft className="size-4" />
        </Button>
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold">{plugin?.name ?? 'Loading...'}</h1>
            {plugin?.version && (
              <Badge variant="secondary">v{plugin.version}</Badge>
            )}
            {plugin?.status && (
              <Badge variant={plugin.status === 'installed' ? 'default' : 'secondary'}>
                {plugin.status}
              </Badge>
            )}
          </div>
          {plugin?.install_path && (
            <p className="text-sm text-muted-foreground">{plugin.install_path}</p>
          )}
        </div>
      </div>

      <ResourceTypeTabs activeTab={activeTab} onTabChange={(v) => setActiveTab(v as import('@/types/v2').ResourceType)} />

      {error && (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-12 text-muted-foreground">
          Loading resources...
        </div>
      ) : filteredResources.length === 0 ? (
        <div className="flex items-center justify-center py-12 text-muted-foreground">
          No resources found
        </div>
      ) : (
        <div className="space-y-2">
          {filteredResources.map((resource) => (
            <div
              key={resource.id}
              className="flex items-center justify-between rounded-lg border p-3 hover:bg-muted/50"
            >
              <div>
                <div className="font-medium">{resource.name}</div>
                <div className="text-sm text-muted-foreground truncate max-w-md">
                  {resource.source_path}
                </div>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => {
                    const filePath = resource.source_path;
                    const extra = resource.resource_type === 'skill'
                      ? `&resource_id=${resource.id}&type=skill&scope=library`
                      : '';
                    navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
                  }}
                  title="Edit"
                >
                  <Pencil className="size-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleExtract(resource.id)}
                  title="Extract to Library"
                >
                  <Archive className="size-4" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
