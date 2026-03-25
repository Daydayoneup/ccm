import { useState } from 'react';
import { Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Checkbox } from '@/components/ui/checkbox';
import type { ResourceLink } from '@/types/v2';

interface ResourceUninstallPopoverProps {
  links: ResourceLink[];
  projectNames: Record<string, string>;
  onUninstall: (linkIds: string[]) => void;
}

export function ResourceUninstallPopover({
  links,
  projectNames,
  onUninstall,
}: ResourceUninstallPopoverProps) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState(false);

  const toggleLink = (linkId: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(linkId)) {
        next.delete(linkId);
      } else {
        next.add(linkId);
      }
      return next;
    });
  };

  const handleUninstall = () => {
    if (selected.size > 0) {
      onUninstall(Array.from(selected));
      setSelected(new Set());
      setOpen(false);
    }
  };

  const getLinkLabel = (link: ResourceLink) => {
    if (link.target_scope === 'global') {
      return '全局 (~/.claude/)';
    }
    if (link.project_id && projectNames[link.project_id]) {
      return `项目: ${projectNames[link.project_id]}`;
    }
    return link.target_path;
  };

  return (
    <Popover open={open} onOpenChange={(v) => { setOpen(v); if (!v) setSelected(new Set()); }}>
      <PopoverTrigger asChild>
        <Button
          variant="ghost"
          size="icon-sm"
          className="text-muted-foreground hover:text-destructive"
          onClick={(e) => e.stopPropagation()}
          title="卸载"
        >
          <Trash2 className="size-3.5" />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-72" onClick={(e) => e.stopPropagation()}>
        <div className="space-y-3">
          <p className="text-sm font-medium">选择要卸载的位置</p>
          <div className="space-y-2">
            {links.map((link) => (
              <label
                key={link.id}
                className="flex cursor-pointer items-center gap-2 text-sm"
              >
                <Checkbox
                  checked={selected.has(link.id)}
                  onCheckedChange={() => toggleLink(link.id)}
                />
                <span className="truncate">{getLinkLabel(link)}</span>
              </label>
            ))}
          </div>
          <Button
            size="sm"
            variant="destructive"
            className="w-full"
            disabled={selected.size === 0}
            onClick={handleUninstall}
          >
            卸载选中 ({selected.size})
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
