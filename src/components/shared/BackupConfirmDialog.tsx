import { useState } from 'react';
import { Link2 } from 'lucide-react';
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

interface BackupConfirmDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: (replaceWithLink: boolean) => void;
  name: string;
  path: string;
}

export function BackupConfirmDialog({
  open,
  onClose,
  onConfirm,
  name,
  path,
}: BackupConfirmDialogProps) {
  const { t } = useI18n();
  const [replaceWithLink, setReplaceWithLink] = useState(false);

  const handleOpenChange = (val: boolean) => {
    if (!val) {
      onClose();
      setReplaceWithLink(false);
    }
  };

  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t('dialogs.backupTitle')}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3">
              <p>
                {t('dialogs.backupPrompt', { name })}
              </p>
              <p className="truncate font-mono text-xs text-muted-foreground">
                {path}
              </p>
              <label className="flex cursor-pointer items-center gap-2 rounded-lg border p-3 transition-colors hover:bg-muted">
                <input
                  type="checkbox"
                  checked={replaceWithLink}
                  onChange={(e) => setReplaceWithLink(e.target.checked)}
                  className="size-4 rounded border-input accent-primary"
                />
                <span className="text-sm font-medium">{t('dialogs.replaceWithLink')}</span>
              </label>
              {replaceWithLink && (
                <div className="flex gap-2 rounded-lg border border-primary/30 bg-primary/5 p-3 text-sm text-primary">
                  <Link2 className="mt-0.5 size-4 shrink-0" />
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    {t('dialogs.replaceWithLinkHint')}
                  </p>
                </div>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
          <AlertDialogAction
            onClick={() => {
              onConfirm(replaceWithLink);
              setReplaceWithLink(false);
            }}
          >
            {t('dialogs.backupConfirm')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
