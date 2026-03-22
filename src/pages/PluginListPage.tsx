import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { usePluginStore } from '@/stores/plugin-store';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Package } from 'lucide-react';

export function PluginListPage() {
  const { plugins, loading, scanning, error, loadPlugins, scanPlugins } = usePluginStore();
  const navigate = useNavigate();

  useEffect(() => {
    loadPlugins();
  }, [loadPlugins]);

  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Plugins</h1>
          <p className="text-sm text-muted-foreground">
            Manage installed Claude Code plugins
          </p>
        </div>
        <Button size="sm" onClick={scanPlugins} disabled={scanning}>
          <RefreshCw className={`mr-1 size-4 ${scanning ? 'animate-spin' : ''}`} />
          {scanning ? 'Scanning...' : 'Scan'}
        </Button>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-12 text-muted-foreground">
          Loading...
        </div>
      ) : plugins.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Package className="mb-4 size-12" />
          <p>No plugins found.</p>
          <p className="text-sm">Click Scan to discover installed plugins.</p>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {plugins.map((plugin) => (
            <Card
              key={plugin.id}
              className="cursor-pointer hover:border-primary/50 transition-colors"
              onClick={() => navigate(`/plugins/${plugin.id}`)}
            >
              <CardHeader className="pb-2">
                <CardTitle className="text-lg">{plugin.name}</CardTitle>
                <CardDescription className="truncate">
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
      )}
    </div>
  );
}
