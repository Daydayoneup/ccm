import { useState, useEffect } from 'react';
import { RefreshCw, Trash2, Plus, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useRegistryStore } from '@/stores/registry-store';
import { useLibraryStore } from '@/stores/library-store-v2';
import { AddRegistryDialog } from '@/components/registry/AddRegistryDialog';

export function LibrarySourcePanel() {
  const { registries, loadRegistries, syncRegistry, removeRegistry, syncing } = useRegistryStore();
  const { selectedSource, setSelectedSource } = useLibraryStore();
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  useEffect(() => {
    loadRegistries();
  }, [loadRegistries]);

  const handleDelete = async (id: string) => {
    if (confirmDeleteId === id) {
      await removeRegistry(id);
      if (selectedSource === id) setSelectedSource('local');
      setConfirmDeleteId(null);
    } else {
      setConfirmDeleteId(id);
    }
  };

  return (
    <div className="flex flex-col gap-1 w-48 shrink-0 border-r border-border/50 pr-3">
      <span className="mb-1 px-3 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground/60">
        Source
      </span>

      {/* Local */}
      <button
        className={cn(
          'flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium text-left transition-all duration-150 w-full',
          selectedSource === 'local'
            ? 'bg-primary/15 text-primary shadow-[inset_3px_0_0_0] shadow-primary'
            : 'text-muted-foreground hover:bg-muted hover:text-foreground'
        )}
        onClick={() => setSelectedSource('local')}
      >
        本地
      </button>

      {/* Registries */}
      {registries.length > 0 && (
        <p className="mt-3 px-3 text-[10px] font-semibold text-muted-foreground/60 uppercase tracking-widest">
          仓库
        </p>
      )}

      {registries.map((registry) => (
        <div key={registry.id} className="group flex items-center gap-1">
          <button
            className={cn(
              'flex-1 truncate rounded-lg px-3 py-2 text-sm text-left transition-all duration-150',
              selectedSource === registry.id
                ? 'bg-primary/15 text-primary shadow-[inset_3px_0_0_0] shadow-primary'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            )}
            onClick={() => setSelectedSource(registry.id)}
            title={registry.url}
          >
            {registry.name}
          </button>
          <Button
            variant="ghost"
            size="icon"
            className="size-7 shrink-0 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-primary"
            onClick={() => syncRegistry(registry.id)}
            disabled={syncing}
            title="同步"
          >
            {syncing ? (
              <Loader2 className="size-3 animate-spin" />
            ) : (
              <RefreshCw className="size-3" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className={cn(
              'size-7 shrink-0 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive',
              confirmDeleteId === registry.id && 'opacity-100 text-destructive'
            )}
            onClick={() => handleDelete(registry.id)}
            title={confirmDeleteId === registry.id ? '再次点击确认删除' : '删除'}
          >
            <Trash2 className="size-3" />
          </Button>
        </div>
      ))}

      {/* Add registry */}
      <Button
        variant="ghost"
        size="sm"
        className="mt-2 justify-start gap-2 rounded-lg px-3 text-muted-foreground hover:text-primary"
        onClick={() => setShowAddDialog(true)}
      >
        <Plus className="size-3" />
        添加仓库
      </Button>

      <AddRegistryDialog
        open={showAddDialog}
        onOpenChange={setShowAddDialog}
      />
    </div>
  );
}
