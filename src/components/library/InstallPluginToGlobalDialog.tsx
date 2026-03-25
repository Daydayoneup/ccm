import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';

interface InstallPluginToGlobalDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  pluginName: string;
  resourceName?: string;
  onConfirm: () => Promise<void>;
}

export function InstallPluginToGlobalDialog({
  open,
  onOpenChange,
  pluginName,
  resourceName,
  onConfirm,
}: InstallPluginToGlobalDialogProps) {
  const [loading, setLoading] = useState(false);

  const handleConfirm = async () => {
    setLoading(true);
    try {
      await onConfirm();
      onOpenChange(false);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>安装到全局</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <p className="text-sm text-muted-foreground">
            {resourceName ? (
              <>确认将资源 <span className="font-medium text-foreground">{resourceName}</span> 安装到全局配置（<code className="text-xs">~/.claude/</code>）？</>
            ) : (
              <>确认将插件 <span className="font-medium text-foreground">{pluginName}</span> 的所有资源安装到全局配置（<code className="text-xs">~/.claude/</code>）？</>
            )}
          </p>
          <p className="text-sm text-muted-foreground">
            全局资源将在所有项目中生效。已存在的同名资源将被跳过。
          </p>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleConfirm} disabled={loading}>
            {loading ? '安装中...' : '确认安装'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
