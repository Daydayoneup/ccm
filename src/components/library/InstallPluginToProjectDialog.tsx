import { useEffect, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useProjectStoreV2 } from '@/stores/project-store-v2';

interface InstallPluginToProjectDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  pluginName: string;
  resourceName?: string;
  onConfirm: (projectId: string) => Promise<void>;
}

export function InstallPluginToProjectDialog({
  open,
  onOpenChange,
  pluginName,
  resourceName,
  onConfirm,
}: InstallPluginToProjectDialogProps) {
  const { projects, loadProjects } = useProjectStoreV2();
  const [selectedProjectId, setSelectedProjectId] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (open) {
      loadProjects();
      setSelectedProjectId('');
    }
  }, [open, loadProjects]);

  const handleConfirm = async () => {
    if (!selectedProjectId) return;
    setLoading(true);
    try {
      await onConfirm(selectedProjectId);
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
          <DialogTitle>安装到项目</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <p className="text-sm text-muted-foreground">
            {resourceName ? (
              <>将资源 <span className="font-medium text-foreground">{resourceName}</span> 安装到项目中。</>
            ) : (
              <>将插件 <span className="font-medium text-foreground">{pluginName}</span> 的所有资源安装到项目中。</>
            )}
          </p>
          <div className="space-y-2">
            <Label>选择项目</Label>
            <Select value={selectedProjectId} onValueChange={setSelectedProjectId}>
              <SelectTrigger>
                <SelectValue placeholder="选择一个项目" />
              </SelectTrigger>
              <SelectContent>
                {projects.map((project) => (
                  <SelectItem key={project.id} value={project.id}>
                    {project.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {projects.length === 0 && (
              <p className="text-xs text-muted-foreground">
                暂无已注册的项目，请先注册项目。
              </p>
            )}
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleConfirm} disabled={loading || !selectedProjectId}>
            {loading ? '安装中...' : '确认安装'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
