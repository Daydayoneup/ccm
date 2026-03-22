import { useEffect, useState } from 'react';
import { Plus, Trash2, Package } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { useLibraryPluginStore } from '@/stores/library-plugin-store';
import { CreateLibraryPluginDialog } from './CreateLibraryPluginDialog';

export function LibraryPluginTab() {
  const { plugins, activePlugin, resources, loadPlugins, createPlugin, deletePlugin, setActivePlugin } =
    useLibraryPluginStore();
  const [showCreate, setShowCreate] = useState(false);

  useEffect(() => {
    loadPlugins();
  }, [loadPlugins]);

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h3 className="text-lg font-medium">插件包</h3>
        <Button onClick={() => setShowCreate(true)}>
          <Plus className="mr-1 h-4 w-4" />
          创建插件包
        </Button>
      </div>

      {!activePlugin ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {plugins.map((plugin) => (
            <Card key={plugin.id} className="cursor-pointer hover:border-primary" onClick={() => setActivePlugin(plugin)}>
              <CardHeader className="pb-2">
                <CardTitle className="text-base flex items-center gap-2">
                  <Package className="h-4 w-4" />
                  {plugin.name}
                </CardTitle>
              </CardHeader>
              <CardContent>
                {plugin.description && (
                  <p className="text-sm text-muted-foreground mb-2">{plugin.description}</p>
                )}
                {plugin.category && <Badge variant="secondary">{plugin.category}</Badge>}
              </CardContent>
            </Card>
          ))}
          {plugins.length === 0 && (
            <p className="col-span-full text-center text-muted-foreground py-8">
              还没有插件包，点击"创建插件包"开始
            </p>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" onClick={() => setActivePlugin(null)}>
                ← 返回
              </Button>
              <h3 className="text-lg font-medium">{activePlugin.name}</h3>
              {activePlugin.category && <Badge variant="secondary">{activePlugin.category}</Badge>}
            </div>
            <Button variant="destructive" size="sm" onClick={() => deletePlugin(activePlugin.id)}>
              <Trash2 className="mr-1 h-4 w-4" />
              删除
            </Button>
          </div>
          {activePlugin.description && (
            <p className="text-sm text-muted-foreground">{activePlugin.description}</p>
          )}
          <div className="space-y-2">
            <h4 className="text-sm font-medium">包内资源 ({resources.length})</h4>
            {resources.map((r) => (
              <div key={r.id} className="flex items-center justify-between p-2 border rounded">
                <div className="flex items-center gap-2">
                  <Badge variant="outline">{r.resource_type}</Badge>
                  <span className="text-sm">{r.name}</span>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => useLibraryPluginStore.getState().removeResource(activePlugin.id, r.id)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            ))}
            {resources.length === 0 && (
              <p className="text-sm text-muted-foreground">
                还没有资源。在资源列表中点击"添加到插件包"来添加资源。
              </p>
            )}
          </div>
        </div>
      )}

      <CreateLibraryPluginDialog
        open={showCreate}
        onOpenChange={setShowCreate}
        onCreate={createPlugin}
      />
    </div>
  );
}
