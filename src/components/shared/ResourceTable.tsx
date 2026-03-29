import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { navigateToResource } from '@/lib/navigation';
import { Archive, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ScopeBadge } from '@/lib/scope-utils';
import { DeleteConfirmDialog } from './DeleteConfirmDialog';
import { BackupConfirmDialog } from './BackupConfirmDialog';
import type { Resource } from '@/types/v2';
import { useI18n } from '@/i18n/provider';

interface ResourceTableProps {
  resources: Resource[];
  onDelete?: (id: string, deleteFromDisk: boolean) => void;
  onBackup?: (id: string, replaceWithLink: boolean) => void;
  showScope?: boolean;
}

/** Determine display scope: if resource was installed from library, show 'library' instead of raw scope */
function getDisplayScope(resource: Resource): string {
  if (resource.installed_from_id) {
    return 'library';
  }
  return resource.scope;
}

const borderClasses: Record<string, string> = {
  skill: 'res-border-skill',
  agent: 'res-border-agent',
  rule: 'res-border-rule',
  hook: 'res-border-hook',
  mcp_server: 'res-border-mcp_server',
  command: 'res-border-command',
};

export function ResourceTable({ resources, onDelete, onBackup, showScope }: ResourceTableProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [pendingDelete, setPendingDelete] = useState<Resource | null>(null);
  const [pendingBackup, setPendingBackup] = useState<Resource | null>(null);

  if (resources.length === 0) {
    return (
      <div className="flex items-center justify-center rounded-md border border-dashed py-16 text-muted-foreground">
        {t('resources.empty')}
      </div>
    );
  }

  return (
    <>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        {resources.map((resource) => {
          return (
            <div
              key={resource.id}
              className={`card-glow group cursor-pointer rounded-md border bg-card/90 p-5 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-[0_20px_50px_rgba(15,23,42,0.10)] ${borderClasses[resource.resource_type] || ''}`}
              onClick={() => navigateToResource(navigate, resource)}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="truncate text-sm font-semibold">{resource.name}</h3>
                    {(showScope || resource.scope !== 'project' || resource.installed_from_id) && (
                      <ScopeBadge scope={getDisplayScope(resource)} className="shrink-0" />
                    )}
                  </div>
                  <p className="mt-2 line-clamp-2 text-sm text-muted-foreground">
                    {resource.description || t('common.noDescription')}
                  </p>
                  <p className="mt-3 truncate text-xs font-mono text-muted-foreground">
                    {resource.source_path}
                  </p>
                </div>
              </div>
              <div className="mt-4 flex items-center justify-end gap-1 border-t border-border/50 pt-3">
                {resource.scope !== 'library' && onBackup && (
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-primary"
                    onClick={(e) => { e.stopPropagation(); setPendingBackup(resource); }}
                    title={t('resources.backupToLibrary')}
                  >
                    <Archive className="size-3.5" />
                  </Button>
                )}
                {onDelete && (
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-destructive"
                    onClick={(e) => { e.stopPropagation(); setPendingDelete(resource); }}
                    title={t('common.delete')}
                  >
                    <Trash2 className="size-3.5" />
                  </Button>
                )}
              </div>
            </div>
          );
        })}
      </div>
      {onDelete && (
        <DeleteConfirmDialog
          open={!!pendingDelete}
          onClose={() => setPendingDelete(null)}
          onConfirm={(deleteFromDisk) => {
            if (pendingDelete) {
              onDelete(pendingDelete.id, deleteFromDisk);
              setPendingDelete(null);
            }
          }}
          title={t('dialogs.deleteResourceTitle')}
          name={pendingDelete?.name ?? ''}
          path={pendingDelete?.source_path ?? ''}
        />
      )}
      {onBackup && (
        <BackupConfirmDialog
          open={!!pendingBackup}
          onClose={() => setPendingBackup(null)}
          onConfirm={async (replaceWithLink) => {
            if (pendingBackup) {
              try {
                await onBackup(pendingBackup.id, replaceWithLink);
              } catch (e) {
                console.error('Backup failed:', e);
                alert(String(e));
              }
              setPendingBackup(null);
            }
          }}
          name={pendingBackup?.name ?? ''}
          path={pendingBackup?.source_path ?? ''}
        />
      )}
    </>
  );
}
