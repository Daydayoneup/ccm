import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useI18n } from '@/i18n/provider';
import { resourceTemplates } from '@/lib/resource-templates';
import type { Resource, ResourceType } from '@/types/v2';

interface CreateResourceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  resourceType: ResourceType;
  onSubmit: (type: ResourceType, name: string, content: string) => Promise<Resource | unknown>;
  onCreated?: (resource: Resource) => void;
}

export function CreateResourceDialog({
  open,
  onOpenChange,
  resourceType,
  onSubmit,
  onCreated,
}: CreateResourceDialogProps) {
  const { t } = useI18n();
  const [name, setName] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    if (!name.trim()) return;
    setLoading(true);
    try {
      const content = resourceTemplates[resourceType](name.trim());
      const result = await onSubmit(resourceType, name.trim(), content);
      setName('');
      onOpenChange(false);
      if (onCreated && result && typeof result === 'object' && 'id' in result) {
        onCreated(result as Resource);
      }
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!loading) onOpenChange(o); }}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>
            {t('projectDetail.createDialogTitle', { type: t(`resourceTypes.${resourceType}`) })}
          </DialogTitle>
        </DialogHeader>
        <div className="py-2">
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter' && name.trim()) handleSubmit(); }}
            placeholder={t('projectDetail.createPlaceholder', { type: resourceType })}
            autoFocus
          />
        </div>
        <DialogFooter>
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t('common.cancel')}
          </Button>
          <Button size="sm" onClick={handleSubmit} disabled={loading || !name.trim()}>
            {loading ? t('projectDetail.creating') : t('common.create')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
