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
import type { LinkType, ResourceType } from '@/types/v2';

interface InstallToProjectDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  resourceName: string;
  resourceType?: ResourceType;
  onInstall: (projectId: string, linkType: LinkType) => Promise<void>;
}

const CONFIG_MERGE_TYPES: ResourceType[] = ['hook', 'mcp_server'];

export function InstallToProjectDialog({
  open,
  onOpenChange,
  resourceName,
  resourceType,
  onInstall,
}: InstallToProjectDialogProps) {
  const { projects, loadProjects } = useProjectStoreV2();
  const [selectedProjectId, setSelectedProjectId] = useState('');
  const [linkType, setLinkType] = useState<LinkType>('symlink');
  const [loading, setLoading] = useState(false);

  const isConfigMerge = resourceType != null && CONFIG_MERGE_TYPES.includes(resourceType);

  useEffect(() => {
    if (open) {
      loadProjects();
      setSelectedProjectId('');
      setLinkType(isConfigMerge ? 'config_merge' : 'symlink');
    }
  }, [open, loadProjects, isConfigMerge]);

  const handleInstall = async () => {
    if (!selectedProjectId) return;
    setLoading(true);
    try {
      await onInstall(selectedProjectId, isConfigMerge ? 'config_merge' : linkType);
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
          <DialogTitle>Install to Project</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <p className="text-sm text-muted-foreground">
            Install <span className="font-medium text-foreground">{resourceName}</span> into a project.
          </p>
          <div className="space-y-2">
            <Label>Project</Label>
            <Select value={selectedProjectId} onValueChange={setSelectedProjectId}>
              <SelectTrigger>
                <SelectValue placeholder="Select a project" />
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
                No projects registered. Register a project first.
              </p>
            )}
          </div>
          {isConfigMerge ? (
            <p className="text-xs text-muted-foreground">
              This resource type uses config merge — the entry will be merged into the project's
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
                  ? 'Creates a symbolic link. Changes to the library resource will be reflected in the project.'
                  : 'Creates an independent copy. Changes to the library resource will not affect the project.'}
              </p>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleInstall} disabled={loading || !selectedProjectId}>
            {loading ? 'Installing...' : 'Install'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
