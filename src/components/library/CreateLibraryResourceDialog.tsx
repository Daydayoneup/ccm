import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Plus } from 'lucide-react';
import type { Resource, ResourceType } from '@/types/v2';

const resourceTemplates: Record<ResourceType, (name: string) => string> = {
  skill: (name) => `---\nname: ${name}\ndescription: \n---\n\n# ${name}\n\n`,
  agent: (name) => `# ${name} Agent\n\nYou are...\n`,
  rule: (name) => `# ${name}\n\n`,
  hook: () => `{\n  "hooks": {}\n}\n`,
  mcp_server: () => `{\n  "mcpServers": {}\n}\n`,
  command: (name) => `# ${name}\n\nUsage: /${name}\n\n`,
};

interface CreateLibraryResourceDialogProps {
  onSubmit: (resourceType: ResourceType, name: string, description: string, content: string) => Promise<Resource>;
  defaultType?: ResourceType;
  onCreated?: (resource: Resource) => void;
}

export function CreateLibraryResourceDialog({ onSubmit, defaultType, onCreated }: CreateLibraryResourceDialogProps) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    if (!name.trim()) return;
    const resourceType = defaultType ?? 'skill';
    setLoading(true);
    try {
      const content = resourceTemplates[resourceType](name.trim());
      const resource = await onSubmit(resourceType, name.trim(), '', content);
      setName('');
      setOpen(false);
      onCreated?.(resource);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm">
          <Plus className="mr-1 size-4" />
          New Resource
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Library Resource</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter' && name.trim()) handleSubmit(); }}
            placeholder="Resource name"
            autoFocus
          />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={loading || !name.trim()}>
            {loading ? 'Creating...' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
