import * as React from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Loader2 } from "lucide-react";
import { forkToLibrary } from "@/lib/tauri-api";

interface ForkToLibraryDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  resourceId: string;
  defaultName: string;
  onForked: (newResourceId: string, newSourcePath: string) => void;
}

export function ForkToLibraryDialog({
  open,
  onOpenChange,
  resourceId,
  defaultName,
  onForked,
}: ForkToLibraryDialogProps) {
  const [name, setName] = React.useState(defaultName);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (open) {
      setName(defaultName);
      setError(null);
    }
  }, [open, defaultName]);

  const handleFork = async () => {
    setLoading(true);
    setError(null);
    try {
      const resource = await forkToLibrary(resourceId, name);
      onForked(resource.id, resource.source_path);
      onOpenChange(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>复制到本地库</DialogTitle>
          <DialogDescription>
            将此 skill 复制到本地资源库，复制后可自由编辑，与原仓库脱钩。
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3 py-2">
          <div className="space-y-1.5">
            <Label htmlFor="fork-name">Skill 名称</Label>
            <Input
              id="fork-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="skill name"
            />
          </div>
          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={loading}>
            取消
          </Button>
          <Button onClick={handleFork} disabled={loading || !name.trim()}>
            {loading && <Loader2 className="mr-2 size-4 animate-spin" />}
            复制
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
