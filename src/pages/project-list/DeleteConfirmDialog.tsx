import { useState } from 'react';
import { AlertTriangle } from 'lucide-react';
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
import { useI18n } from '@/i18n/provider';
import type { Project } from '@/types/v2';

interface DeleteConfirmDialogProps {
  project: Project | null;
  onClose: () => void;
  onConfirm: (id: string, fromDisk: boolean) => void;
}

export function DeleteConfirmDialog({ project, onClose, onConfirm }: DeleteConfirmDialogProps) {
  const { t } = useI18n();
  const [deleteFromDisk, setDeleteFromDisk] = useState(false);

  return (
    <AlertDialog open={!!project} onOpenChange={(open) => { if (!open) { onClose(); setDeleteFromDisk(false); } }}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t('projects.deleteTitle')}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3">
              <p>{project ? t('projects.deletePrompt', { name: project.name }) : ''}</p>
              <p className="truncate font-mono text-xs text-muted-foreground">{project?.path}</p>
              <label className="flex cursor-pointer items-center gap-2 rounded-lg border p-3 transition-colors hover:bg-muted">
                <input
                  type="checkbox"
                  checked={deleteFromDisk}
                  onChange={(e) => setDeleteFromDisk(e.target.checked)}
                  className="size-4 rounded border-input accent-destructive"
                />
                <span className="text-sm font-medium">{t('projects.deleteFromDisk')}</span>
              </label>
              {deleteFromDisk ? (
                <div className="flex gap-2 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                  <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                  <div>
                    <p className="font-medium">{t('projects.deleteWarningTitle')}</p>
                    <p className="mt-1 text-xs leading-relaxed text-destructive/80">{t('projects.deleteWarningBody')}</p>
                  </div>
                </div>
              ) : null}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
          <AlertDialogAction
            className={deleteFromDisk ? 'bg-destructive text-destructive-foreground hover:bg-destructive/90' : ''}
            onClick={async () => {
              if (project) {
                await onConfirm(project.id, deleteFromDisk);
                setDeleteFromDisk(false);
              }
            }}
          >
            {t('common.confirmDelete')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
