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
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPos, setMenuPos] = useState({ top: 0, left: 0 });

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const handleToggle = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!open) {
      // Use the click target (the button element) to compute position
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      setMenuPos({
        top: rect.bottom + 4,
        left: rect.right,
      });
    }
    setOpen(!open);
  };

  return (
    <>
      <Button
        variant="ghost"
        size="icon-sm"
        className="text-muted-foreground hover:text-primary"
        onClick={handleToggle}
      >
        <Download className="size-3.5" />
      </Button>
      {open && (
        <div
          ref={menuRef}
          className="fixed z-[9999] min-w-[140px] rounded-md border bg-popover p-1 shadow-md"
          style={{ top: menuPos.top, left: menuPos.left, transform: 'translateX(-100%)' }}
        >
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
    </>
  );
}
