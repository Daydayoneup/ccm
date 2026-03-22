import { useState } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

interface CreateLibraryPluginDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreate: (name: string, description: string | null, category: string | null) => Promise<void>;
}

const CATEGORIES = ['development', 'productivity', 'security', 'testing', 'monitoring', 'other'];

export function CreateLibraryPluginDialog({ open, onOpenChange, onCreate }: CreateLibraryPluginDialogProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState('');
  const [loading, setLoading] = useState(false);

  const handleCreate = async () => {
    if (!name.trim()) return;
    setLoading(true);
    try {
      await onCreate(name.trim(), description.trim() || null, category || null);
      setName('');
      setDescription('');
      setCategory('');
      onOpenChange(false);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>创建插件包</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div>
            <Label htmlFor="plugin-name">名称</Label>
            <Input id="plugin-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="my-plugin-pack" />
          </div>
          <div>
            <Label htmlFor="plugin-desc">描述</Label>
            <Input id="plugin-desc" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="可选" />
          </div>
          <div>
            <Label>分类</Label>
            <Select value={category} onValueChange={setCategory}>
              <SelectTrigger>
                <SelectValue placeholder="选择分类（可选）" />
              </SelectTrigger>
              <SelectContent>
                {CATEGORIES.map((cat) => (
                  <SelectItem key={cat} value={cat}>{cat}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleCreate} disabled={!name.trim() || loading}>
            {loading ? '创建中...' : '创建'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
