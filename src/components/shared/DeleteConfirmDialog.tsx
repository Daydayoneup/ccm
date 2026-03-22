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
                确定要删除 <span className="font-semibold text-foreground">{name}</span> 吗？
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
                <span className="text-sm font-medium">同时删除磁盘文件（不可恢复！）</span>
              </label>
              {deleteFromDisk && (
                <div className="flex gap-2 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                  <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                  <div>
                    <p className="font-medium">警告：此操作无法撤销</p>
                    <p className="mt-1 text-xs leading-relaxed text-destructive/80">
                      将永久删除磁盘上的文件。此操作不可恢复。
                    </p>
                  </div>
                </div>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            className={deleteFromDisk ? 'bg-destructive text-destructive-foreground hover:bg-destructive/90' : ''}
            onClick={() => {
              onConfirm(deleteFromDisk);
              setDeleteFromDisk(false);
            }}
          >
            确认删除
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
