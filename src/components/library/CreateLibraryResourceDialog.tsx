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
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Plus } from 'lucide-react';
import type { ResourceType } from '@/types/v2';

interface CreateLibraryResourceDialogProps {
  onSubmit: (resourceType: ResourceType, name: string, description: string, content: string) => Promise<unknown>;
  defaultType?: ResourceType;
}

export function CreateLibraryResourceDialog({ onSubmit, defaultType }: CreateLibraryResourceDialogProps) {
  const [open, setOpen] = useState(false);
  const [resourceType, setResourceType] = useState<ResourceType>(defaultType ?? 'skill');
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    if (!name.trim()) return;
    setLoading(true);
    try {
      await onSubmit(resourceType, name.trim(), description.trim(), content);
      setName('');
      setDescription('');
      setContent('');
      setOpen(false);
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
          <div className="space-y-2">
            <Label>Type</Label>
            <Select value={resourceType} onValueChange={(v) => setResourceType(v as ResourceType)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="skill">Skill</SelectItem>
                <SelectItem value="agent">Agent</SelectItem>
                <SelectItem value="rule">Rule</SelectItem>
                <SelectItem value="hook">Hook</SelectItem>
                <SelectItem value="mcp_server">MCP Server</SelectItem>
                <SelectItem value="command">Command (Legacy)</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label>Name</Label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-resource"
            />
          </div>
          <div className="space-y-2">
            <Label>Description</Label>
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="A brief description of this resource"
            />
          </div>
          <div className="space-y-2">
            <Label>Content</Label>
            <textarea
              className="flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder={resourceType === 'hook' ? '{ "hooks": [] }' : '# Resource Name\n\nDescription...'}
            />
          </div>
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
