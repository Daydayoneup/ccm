import { useEffect } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useLibraryPluginStore } from '@/stores/library-plugin-store';

interface AddToPluginDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  resourceId: string;
}

export function AddToPluginDialog({ open, onOpenChange, resourceId }: AddToPluginDialogProps) {
  const { plugins, loadPlugins, addResource } = useLibraryPluginStore();

  useEffect(() => {
    if (open) loadPlugins();
  }, [open, loadPlugins]);

  const handleAdd = async (pluginId: string) => {
    await addResource(pluginId, resourceId);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>添加到插件包</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          {plugins.map((plugin) => (
            <Button
              key={plugin.id}
              variant="outline"
              className="w-full justify-start"
              onClick={() => handleAdd(plugin.id)}
            >
              {plugin.name}
              {plugin.category && <span className="ml-2 text-muted-foreground">({plugin.category})</span>}
            </Button>
          ))}
          {plugins.length === 0 && (
            <p className="text-sm text-muted-foreground text-center py-4">
              还没有插件包。请先创建一个插件包。
            </p>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
