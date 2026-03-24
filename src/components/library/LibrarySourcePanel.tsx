import { useEffect, useState } from 'react';
import { Loader2, Plus, RefreshCw, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useRegistryStore } from '@/stores/registry-store';
import { useLibraryStore } from '@/stores/library-store-v2';
import { AddRegistryDialog } from '@/components/registry/AddRegistryDialog';
import { useI18n } from '@/i18n/provider';

export function LibrarySourcePanel() {
  const { t } = useI18n();
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
      return;
    }
    setConfirmDeleteId(id);
  };

  return (
    <div className="w-64 shrink-0 rounded-md border border-border/60 bg-card/88 p-3 shadow-[0_12px_32px_rgba(15,23,42,0.08)]">
      <span className="mb-2 block px-2 text-[10px] font-semibold uppercase tracking-[0.22em] text-muted-foreground/65">
        {t('library.sources')}
      </span>

      <button
        className={cn(
          'flex w-full items-center gap-2 rounded-md px-3 py-3 text-left text-sm font-medium transition-all duration-150',
          selectedSource === 'local'
            ? 'bg-primary/14 text-primary shadow-[inset_3px_0_0_0] shadow-primary'
            : 'text-muted-foreground hover:bg-muted hover:text-foreground'
        )}
        onClick={() => setSelectedSource('local')}
      >
        {t('library.localSource')}
      </button>

      {registries.length > 0 && (
        <p className="mt-4 px-2 text-[10px] font-semibold uppercase tracking-[0.22em] text-muted-foreground/65">
          {t('library.registries')}
        </p>
      )}

      <div className="mt-2 space-y-1">
        {registries.map((registry) => (
          <div key={registry.id} className="group flex items-center gap-1">
            <button
              className={cn(
                'flex-1 truncate rounded-md px-3 py-3 text-left text-sm transition-all duration-150',
                selectedSource === registry.id
                  ? 'bg-primary/14 text-primary shadow-[inset_3px_0_0_0] shadow-primary'
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
              className="size-8 shrink-0 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-primary"
              onClick={() => syncRegistry(registry.id)}
              disabled={syncing}
              title={t('library.syncRegistry')}
            >
              {syncing ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className={cn(
                'size-8 shrink-0 opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive',
                confirmDeleteId === registry.id && 'opacity-100 text-destructive'
              )}
              onClick={() => handleDelete(registry.id)}
              title={t('library.removeRegistry')}
            >
              <Trash2 className="size-3.5" />
            </Button>
          </div>
        ))}
      </div>

      <Button
        variant="ghost"
        size="sm"
        className="mt-3 w-full justify-start gap-2 rounded-md px-3 text-muted-foreground hover:text-primary"
        onClick={() => setShowAddDialog(true)}
      >
        <Plus className="size-3.5" />
        {t('library.addRegistry')}
      </Button>

      <AddRegistryDialog open={showAddDialog} onOpenChange={setShowAddDialog} />
    </div>
  );
}
