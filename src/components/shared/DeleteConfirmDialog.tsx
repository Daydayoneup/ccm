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

interface DeleteConfirmDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: (deleteFromDisk: boolean) => void;
  title: string;
  name: string;
  path: string;
}

export function DeleteConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  name,
  path,
}: DeleteConfirmDialogProps) {
  const { t } = useI18n();
  const [deleteFromDisk, setDeleteFromDisk] = useState(false);

  const handleOpenChange = (val: boolean) => {
    if (!val) {
      onClose();
      setDeleteFromDisk(false);
    }
  };

  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3">
              <p>
                {t('dialogs.deletePrompt', { name })}
              </p>
              <p className="truncate font-mono text-xs text-muted-foreground">
                {path}
              </p>
              <label className="flex cursor-pointer items-center gap-2 rounded-lg border p-3 transition-colors hover:bg-muted">
                <input
                  type="checkbox"
                  checked={deleteFromDisk}
                  onChange={(e) => setDeleteFromDisk(e.target.checked)}
                  className="size-4 rounded border-input accent-destructive"
                />
                <span className="text-sm font-medium">{t('dialogs.deleteFromDisk')}</span>
              </label>
              {deleteFromDisk && (
                <div className="flex gap-2 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                  <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                  <div>
                    <p className="font-medium">{t('dialogs.deleteWarningTitle')}</p>
                    <p className="mt-1 text-xs leading-relaxed text-destructive/80">
                      {t('dialogs.deleteWarningBody')}
                    </p>
                  </div>
                </div>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
          <AlertDialogAction
            className={deleteFromDisk ? 'bg-destructive text-destructive-foreground hover:bg-destructive/90' : ''}
            onClick={() => {
              onConfirm(deleteFromDisk);
              setDeleteFromDisk(false);
            }}
          >
            {t('common.confirmDelete')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
