import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
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
import { useRegistryStore } from '@/stores/registry-store';

interface AddRegistryDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AddRegistryDialog({ open, onOpenChange }: AddRegistryDialogProps) {
  const { addRegistry } = useRegistryStore();
  const [url, setUrl] = useState('');
  const [name, setName] = useState('');
  const [readonly, setReadonly] = useState('true');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    if (!url.trim()) {
      setError('URL is required');
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await addRegistry(name.trim(), url.trim(), readonly === 'true');
      setUrl('');
      setName('');
      setReadonly('true');
      onOpenChange(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add Registry</DialogTitle>
          <DialogDescription>
            Add a Git repository as a resource registry.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="registry-url">URL</Label>
            <Input
              id="registry-url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://github.com/user/repo.git"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="registry-name">Name</Label>
            <Input
              id="registry-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="留空则使用仓库名"
            />
          </div>
          <div className="space-y-2">
            <Label>Access Mode</Label>
            <Select value={readonly} onValueChange={setReadonly}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="true">只读</SelectItem>
                <SelectItem value="false">读写</SelectItem>
              </SelectContent>
            </Select>
          </div>
          {error && (
            <div className="rounded-lg border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={loading || !url.trim()}>
            {loading ? 'Adding...' : 'Add'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
