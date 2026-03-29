import { useState } from 'react';
import { updateMcpServerConfig, createMcpServer, deleteProjectResource } from '@/lib/tauri-api';
import { Pencil, Plus, Server, Trash2 } from 'lucide-react';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { ScopeBadge } from '@/lib/scope-utils';
import { useI18n } from '@/i18n/provider';
import type { Resource } from '@/types/v2';

interface McpServerListProps {
  resources: Resource[];
  emptyMessage?: string;
  emptyHint?: string;
  /** Project ID — required for creating new MCP servers */
  projectId?: string;
  /** Called after a create/edit to refresh the resource list */
  onRefresh?: () => void;
}

/** Parse MCP server details from Resource.metadata JSON */
function parseMcpMeta(resource: Resource) {
  if (!resource.metadata) return {};
  try {
    return JSON.parse(resource.metadata) as Record<string, unknown>;
  } catch {
    return {};
  }
}

function formatJson(value: unknown): string {
  try {
    return JSON.stringify(typeof value === 'string' ? JSON.parse(value) : value, null, 2);
  } catch {
    return String(value ?? '{\n  \n}');
  }
}

export function McpServerList({
  resources,
  emptyMessage,
  emptyHint,
  projectId,
  onRefresh,
}: McpServerListProps) {
  const { t } = useI18n();
  const [editingResource, setEditingResource] = useState<Resource | null>(null);
  const [editJson, setEditJson] = useState('');
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const [newJson, setNewJson] = useState('{\n  \n}');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingDeleteResource, setPendingDeleteResource] = useState<Resource | null>(null);

  const handleEdit = (resource: Resource) => {
    setEditingResource(resource);
    setEditJson(resource.metadata ? formatJson(resource.metadata) : '{\n  \n}');
    setError(null);
  };

  const handleSaveEdit = async () => {
    if (!editingResource) return;
    setSaving(true);
    setError(null);
    try {
      // Validate JSON first
      JSON.parse(editJson);
      await updateMcpServerConfig(editingResource.id, editJson);
      setEditingResource(null);
      onRefresh?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleCreate = async () => {
    if (!projectId || !newName.trim()) return;
    setSaving(true);
    setError(null);
    try {
      JSON.parse(newJson);
      await createMcpServer(projectId, newName.trim(), newJson);
      setCreating(false);
      setNewName('');
      setNewJson('{\n  \n}');
      onRefresh?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      {resources.length === 0 && !creating ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Server className="mb-4 size-12" />
          <p>{emptyMessage ?? t('mcp.empty')}</p>
          {emptyHint ? <p className="text-sm">{emptyHint}</p> : null}
          {projectId ? (
            <Button variant="outline" size="sm" className="mt-4" onClick={() => { setCreating(true); setError(null); }}>
              <Plus className="mr-1 size-3.5" />
              {t('mcp.create')}
            </Button>
          ) : null}
        </div>
      ) : (
        <div className="space-y-3">
          {resources.map((resource) => {
            const meta = parseMcpMeta(resource);
            const serverType = (meta.type ?? meta.server_type ?? null) as string | null;
            const command = (meta.command ?? null) as string | null;
            const args = meta.args as string[] | null;
            const url = (meta.url ?? null) as string | null;

            return (
              <div
                key={resource.id}
                className="group rounded-[20px] border p-4 transition-colors hover:bg-muted/30"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Server className="size-4 text-muted-foreground" />
                    <span className="font-medium">{resource.name}</span>
                    <ScopeBadge scope={resource.scope} />
                  </div>
                  <div className="flex items-center gap-2">
                    {serverType ? <Badge variant="secondary">{serverType}</Badge> : null}
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      className="opacity-0 transition-opacity group-hover:opacity-100"
                      onClick={() => handleEdit(resource)}
                      title={t('mcp.edit')}
                    >
                      <Pencil className="size-3.5" />
                    </Button>
                    {projectId && (
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="opacity-0 transition-opacity group-hover:opacity-100 text-muted-foreground hover:text-destructive"
                        onClick={() => setPendingDeleteResource(resource)}
                        title={t('common.delete')}
                      >
                        <Trash2 className="size-3.5" />
                      </Button>
                    )}
                  </div>
                </div>
                <div className="mt-2 space-y-1 text-sm text-muted-foreground">
                  {command ? (
                    <p>
                      <span className="font-medium">{t('mcp.command')}:</span> {command}{' '}
                      {args ? (Array.isArray(args) ? args.join(' ') : String(args)) : ''}
                    </p>
                  ) : null}
                  {url ? (
                    <p>
                      <span className="font-medium">{t('mcp.url')}:</span> {url}
                    </p>
                  ) : null}
                  <p className="truncate">
                    <span className="font-medium">{t('mcp.source')}:</span> {resource.source_path}
                  </p>
                </div>
              </div>
            );
          })}
          {projectId ? (
            <Button variant="outline" size="sm" onClick={() => { setCreating(true); setError(null); }}>
              <Plus className="mr-1 size-3.5" />
              {t('mcp.create')}
            </Button>
          ) : null}
        </div>
      )}

      {/* Edit Dialog */}
      <Dialog open={!!editingResource} onOpenChange={(open) => { if (!open) setEditingResource(null); }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t('mcp.editTitle', { name: editingResource?.name ?? '' })}</DialogTitle>
          </DialogHeader>
          <textarea
            className="h-64 w-full rounded-md border bg-muted/50 p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-primary"
            value={editJson}
            onChange={(e) => setEditJson(e.target.value)}
            spellCheck={false}
          />
          {error ? (
            <p className="text-sm text-destructive">{error}</p>
          ) : null}
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setEditingResource(null)}>
              {t('common.cancel')}
            </Button>
            <Button size="sm" onClick={handleSaveEdit} disabled={saving}>
              {saving ? t('common.saving') : t('common.save')}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Create Dialog */}
      <Dialog open={creating} onOpenChange={(open) => { if (!open) setCreating(false); }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t('mcp.createTitle')}</DialogTitle>
          </DialogHeader>
          <div className="space-y-3">
            <Input
              placeholder={t('mcp.namePlaceholder')}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              autoFocus
            />
            <textarea
              className="h-48 w-full rounded-md border bg-muted/50 p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-primary"
              value={newJson}
              onChange={(e) => setNewJson(e.target.value)}
              spellCheck={false}
              placeholder='{ "command": "npx", "args": ["..."] }'
            />
          </div>
          {error ? (
            <p className="text-sm text-destructive">{error}</p>
          ) : null}
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="sm" onClick={() => setCreating(false)}>
              {t('common.cancel')}
            </Button>
            <Button size="sm" onClick={handleCreate} disabled={saving || !newName.trim()}>
              {saving ? t('common.saving') : t('common.create')}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      <AlertDialog open={!!pendingDeleteResource} onOpenChange={(open) => { if (!open) setPendingDeleteResource(null); }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('common.confirmDelete')}</AlertDialogTitle>
            <AlertDialogDescription>
              {pendingDeleteResource ? t('mcp.confirmDelete', { name: pendingDeleteResource.name }) : ''}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={async () => {
                if (pendingDeleteResource) {
                  await deleteProjectResource(pendingDeleteResource.id);
                  setPendingDeleteResource(null);
                  onRefresh?.();
                }
              }}
            >
              {t('common.confirmDelete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
