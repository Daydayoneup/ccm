import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Trash2, Archive } from 'lucide-react';
import { ScopeBadge } from '@/lib/scope-utils';
import { DeleteConfirmDialog } from './DeleteConfirmDialog';
import type { Resource } from '@/types/v2';
import { useNavigate } from 'react-router-dom';

interface ResourceTableProps {
  resources: Resource[];
  onDelete?: (id: string, deleteFromDisk: boolean) => void;
  onBackup?: (id: string) => void;
  showScope?: boolean;
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
  const navigate = useNavigate();
  const [pendingDelete, setPendingDelete] = useState<Resource | null>(null);

  if (resources.length === 0) {
    return (
      <div className="flex items-center justify-center rounded-xl border border-dashed py-16 text-muted-foreground">
        No resources found
      </div>
    );
  }

  return (
    <>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
        {resources.map((resource) => {
          const isGlobal = resource.scope === 'global';
          return (
            <div
              key={resource.id}
              className={`card-glow group cursor-pointer rounded-xl border bg-card p-4 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/5 ${borderClasses[resource.resource_type] || ''}`}
              onClick={() => navigate(`/editor?file=${encodeURIComponent(resource.source_path)}`)}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="truncate font-semibold text-sm">{resource.name}</h3>
                    {(showScope || resource.scope !== 'project') && (
                      <ScopeBadge scope={resource.scope} className="shrink-0" />
                    )}
                  </div>
                  <p className="mt-1.5 truncate text-xs font-mono text-muted-foreground">
                    {resource.source_path}
                  </p>
                </div>
              </div>
              <div className="mt-3 flex items-center justify-end gap-0.5 border-t border-border/50 pt-3">
                {!isGlobal && onBackup && (
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-primary"
                    onClick={(e) => { e.stopPropagation(); onBackup(resource.id); }}
                    title="Backup to Library"
                  >
                    <Archive className="size-3.5" />
                  </Button>
                )}
                {!isGlobal && onDelete && (
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-destructive"
                    onClick={(e) => { e.stopPropagation(); setPendingDelete(resource); }}
                    title="Delete"
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
          title="确认删除资源"
          name={pendingDelete?.name ?? ''}
          path={pendingDelete?.source_path ?? ''}
        />
      )}
    </>
  );
}
