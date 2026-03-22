import { useState } from 'react';
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
import type { LinkType, ResourceType } from '@/types/v2';

interface DeployToGlobalDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  resourceName: string;
  resourceType?: ResourceType;
  onDeploy: (linkType: LinkType) => Promise<void>;
}

const CONFIG_MERGE_TYPES: ResourceType[] = ['hook', 'mcp_server'];

export function DeployToGlobalDialog({
  open,
  onOpenChange,
  resourceName,
  resourceType,
  onDeploy,
}: DeployToGlobalDialogProps) {
  const isConfigMerge = resourceType != null && CONFIG_MERGE_TYPES.includes(resourceType);
  const [linkType, setLinkType] = useState<LinkType>('symlink');
  const [loading, setLoading] = useState(false);

  const handleDeploy = async () => {
    setLoading(true);
    try {
      await onDeploy(isConfigMerge ? 'config_merge' : linkType);
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
          <DialogTitle>Deploy to Global</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <p className="text-sm text-muted-foreground">
            Deploy <span className="font-medium text-foreground">{resourceName}</span> to your
            global Claude configuration (<code className="text-xs">~/.claude/</code>).
          </p>
          <p className="text-sm text-muted-foreground">
            Global resources are active across all projects. This will make the resource available
            everywhere Claude Code runs on your machine.
          </p>
          {isConfigMerge ? (
            <p className="text-xs text-muted-foreground">
              This resource type uses config merge — the entry will be merged into the global
              config file automatically.
            </p>
          ) : (
            <div className="space-y-2">
              <Label>Link Type</Label>
              <Select value={linkType} onValueChange={(v) => setLinkType(v as LinkType)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="symlink">Symlink (recommended)</SelectItem>
                  <SelectItem value="copy">Copy</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {linkType === 'symlink'
                  ? 'Creates a symbolic link. Changes to the library resource will be reflected globally.'
                  : 'Creates an independent copy. Changes to the library resource will not affect the global copy.'}
              </p>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleDeploy} disabled={loading}>
            {loading ? 'Deploying...' : 'Deploy'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
