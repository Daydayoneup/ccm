import { useState, useRef, useEffect } from 'react';
import { Download, FolderOpen, Globe } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface ResourceInstallDropdownProps {
  onInstallToProject: () => void;
  onInstallToGlobal: () => void;
}

export function ResourceInstallDropdown({
  onInstallToProject,
  onInstallToGlobal,
}: ResourceInstallDropdownProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  return (
    <div ref={ref} className="relative">
      <Button
        variant="ghost"
        size="icon-sm"
        className="text-muted-foreground hover:text-primary"
        onClick={(e) => {
          e.stopPropagation();
          setOpen(!open);
        }}
      >
        <Download className="size-3.5" />
      </Button>
      {open && (
        <div className="absolute right-0 top-full z-50 mt-1 min-w-[140px] rounded-md border bg-popover p-1 shadow-md">
          <button
            className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground"
            onClick={(e) => {
              e.stopPropagation();
              setOpen(false);
              onInstallToProject();
            }}
          >
            <FolderOpen className="h-4 w-4" />
            安装到项目
          </button>
          <button
            className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground"
            onClick={(e) => {
              e.stopPropagation();
              setOpen(false);
              onInstallToGlobal();
            }}
          >
            <Globe className="h-4 w-4" />
            安装到全局
          </button>
        </div>
      )}
    </div>
  );
}
