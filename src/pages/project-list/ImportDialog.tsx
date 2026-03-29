import { useEffect, useState } from 'react';
import { Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { useI18n } from '@/i18n/provider';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import type { DiscoveredProject } from '@/types/v2';

interface ImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImported: () => void;
}

export function ImportDialog({ open, onOpenChange, onImported }: ImportDialogProps) {
  const { t, formatNumber } = useI18n();
  const { claudeDiscoveredProjects, discoverFromClaude, registerProject } = useProjectStoreV2();
  const [importLoading, setImportLoading] = useState(false);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [importProgress, setImportProgress] = useState<{ current: number; total: number } | null>(null);

  useEffect(() => {
    if (!open) return;
    setSelectedPaths(new Set());
    setImportProgress(null);
    setImportLoading(true);
    discoverFromClaude().finally(() => setImportLoading(false));
  }, [open, discoverFromClaude]);

  const handleImportSelected = async () => {
    const paths = Array.from(selectedPaths);
    if (paths.length === 0) return;
    setImportProgress({ current: 0, total: paths.length });
    for (let index = 0; index < paths.length; index += 1) {
      setImportProgress({ current: index + 1, total: paths.length });
      try {
        await registerProject(paths[index]);
      } catch {
        // Continue importing remaining projects.
      }
    }
    setImportProgress(null);
    onOpenChange(false);
    onImported();
  };

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!importProgress) onOpenChange(o); }}>
      <DialogContent className="flex max-h-[85vh] flex-col overflow-hidden p-0 sm:max-w-2xl">
        <div className="flex items-center justify-between border-b px-5 py-4">
          <div>
            <DialogTitle className="text-base font-semibold">{t('projects.importDialogTitle')}</DialogTitle>
            {!importLoading && claudeDiscoveredProjects.length > 0 && (
              <p className="mt-1 text-xs text-muted-foreground">
                {t('projects.discoveredProjects', { count: formatNumber(claudeDiscoveredProjects.length) })}
              </p>
            )}
          </div>
          {!importLoading && claudeDiscoveredProjects.length > 0 && (
            <label className="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground">
              <input
                type="checkbox"
                checked={selectedPaths.size === claudeDiscoveredProjects.length && claudeDiscoveredProjects.length > 0}
                onChange={() =>
                  setSelectedPaths(
                    selectedPaths.size === claudeDiscoveredProjects.length
                      ? new Set()
                      : new Set(claudeDiscoveredProjects.map((project: DiscoveredProject) => project.path))
                  )
                }
                className="size-3.5 rounded border-input accent-primary"
              />
              {t('projects.selectAll')}
            </label>
          )}
        </div>

        <div className="flex-1 overflow-y-auto px-2 py-1">
          {importLoading ? (
            <div className="flex items-center justify-center py-16 text-muted-foreground">
              <Loader2 className="mr-2 size-5 animate-spin text-primary" />
              {t('projects.discovering')}
            </div>
          ) : claudeDiscoveredProjects.length === 0 ? (
            <p className="py-16 text-center text-sm text-muted-foreground">
              {t('projects.noClaudeProjects')}
            </p>
          ) : (
            <div className="space-y-px">
              {claudeDiscoveredProjects.map((project) => (
                <label
                  key={project.path}
                  className="flex cursor-pointer items-center gap-3 rounded-md px-3 py-2 transition-colors hover:bg-accent/30"
                >
                  <input
                    type="checkbox"
                    checked={selectedPaths.has(project.path)}
                    onChange={() => {
                      setSelectedPaths((prev) => {
                        const next = new Set(prev);
                        if (next.has(project.path)) next.delete(project.path);
                        else next.add(project.path);
                        return next;
                      });
                    }}
                    className="size-3.5 shrink-0 rounded border-input accent-primary"
                  />
                  <span className="shrink-0 text-sm font-medium">{project.name}</span>
                  {project.has_claude_config ? (
                    <span className="shrink-0 rounded bg-primary/10 px-1.5 py-px text-[10px] font-medium text-primary">.claude</span>
                  ) : null}
                  <span className="min-w-0 flex-1 truncate text-right font-mono text-[11px] text-muted-foreground/60">
                    {project.path}
                  </span>
                </label>
              ))}
            </div>
          )}
        </div>

        {!importLoading && claudeDiscoveredProjects.length > 0 && (
          <div className="flex items-center justify-between border-t bg-muted/30 px-5 py-3">
            <div className="text-xs text-muted-foreground tabular-nums">
              {importProgress ? (
                <span className="flex items-center gap-1.5">
                  <Loader2 className="size-3 animate-spin text-primary" />
                  {t('projects.importingProgress', importProgress)}
                </span>
              ) : (
                t('common.selectedCount', {
                  selected: formatNumber(selectedPaths.size),
                  total: formatNumber(claudeDiscoveredProjects.length),
                })
              )}
            </div>
            <Button
              size="sm"
              className="rounded-md"
              onClick={handleImportSelected}
              disabled={selectedPaths.size === 0 || importProgress !== null}
            >
              {importProgress ? t('projects.importing') : t('projects.importSelected', { count: formatNumber(selectedPaths.size) })}
            </Button>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
