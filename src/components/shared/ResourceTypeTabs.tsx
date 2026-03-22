import { useRef, useState, useEffect, useCallback } from 'react';
import { cn } from '@/lib/utils';
import type { ResourceType } from '@/types/v2';

const RESOURCE_TYPES: { value: ResourceType; label: string; dotColor: string }[] = [
  { value: 'skill', label: 'Skills', dotColor: 'bg-res-skill' },
  { value: 'agent', label: 'Agents', dotColor: 'bg-res-agent' },
  { value: 'rule', label: 'Rules', dotColor: 'bg-res-rule' },
  { value: 'hook', label: 'Hooks', dotColor: 'bg-res-hook' },
  { value: 'mcp_server', label: 'MCP', dotColor: 'bg-res-mcp' },
  { value: 'command', label: 'Cmds', dotColor: 'bg-res-command' },
];

const EXTRA_TABS: { value: string; label: string }[] = [
  { value: 'mcp', label: 'MCP Servers' },
  { value: 'plugin', label: 'Plugins' },
  { value: 'permissions', label: 'Perms' },
  { value: 'env', label: 'Env' },
  { value: 'files', label: 'Files' },
];

interface ResourceTypeTabsProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
  includeMcp?: boolean;
  includePlugin?: boolean;
  includePermissions?: boolean;
  includeEnv?: boolean;
  includeFiles?: boolean;
  counts?: Partial<Record<string, number>>;
}

export function ResourceTypeTabs({ activeTab, onTabChange, includeMcp, includePlugin, includePermissions, includeEnv, includeFiles, counts }: ResourceTypeTabsProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  const extras = EXTRA_TABS.filter(
    (t) =>
      (t.value === 'mcp' && includeMcp) ||
      (t.value === 'plugin' && includePlugin) ||
      (t.value === 'permissions' && includePermissions) ||
      (t.value === 'env' && includeEnv) ||
      (t.value === 'files' && includeFiles)
  );
  const extrasWithoutFiles = extras.filter((t) => t.value !== 'files');

  const checkScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    setCanScrollLeft(el.scrollLeft > 2);
    setCanScrollRight(el.scrollLeft < el.scrollWidth - el.clientWidth - 2);
  }, []);

  useEffect(() => {
    checkScroll();
    const el = scrollRef.current;
    if (!el) return;
    el.addEventListener('scroll', checkScroll, { passive: true });
    const ro = new ResizeObserver(checkScroll);
    ro.observe(el);
    return () => {
      el.removeEventListener('scroll', checkScroll);
      ro.disconnect();
    };
  }, [checkScroll]);

  return (
    <div className="relative min-w-0 flex-1">
      {/* Left fade */}
      {canScrollLeft && (
        <div className="pointer-events-none absolute left-0 top-0 bottom-0 z-10 w-8 bg-gradient-to-r from-background to-transparent" />
      )}
      {/* Right fade */}
      {canScrollRight && (
        <div className="pointer-events-none absolute right-0 top-0 bottom-0 z-10 w-8 bg-gradient-to-l from-background to-transparent" />
      )}

      <div
        ref={scrollRef}
        className="flex items-center gap-1 overflow-x-auto scrollbar-none"
        style={{ scrollbarWidth: 'none' }}
      >
        {/* Files tab first if included */}
        {includeFiles && (
          <button
            onClick={() => onTabChange('files')}
            className={cn(
              'shrink-0 rounded-lg px-2.5 py-1.5 text-xs font-medium transition-all duration-150',
              activeTab === 'files'
                ? 'bg-primary/15 text-primary shadow-sm'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            )}
          >
            Files
          </button>
        )}
        {includeFiles && (
          <div className="mx-0.5 h-5 w-px shrink-0 bg-border" />
        )}

        {/* Resource type tabs */}
        {RESOURCE_TYPES.map((type) => {
          const count = counts?.[type.value];
          return (
            <button
              key={type.value}
              onClick={() => onTabChange(type.value)}
              className={cn(
                'flex shrink-0 items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs font-medium transition-all duration-150',
                activeTab === type.value
                  ? 'bg-primary/15 text-primary shadow-sm'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'
              )}
            >
              <span className={cn('size-1.5 rounded-full shrink-0', type.dotColor)} />
              {type.label}
              {count != null && count > 0 && (
                <span className={cn(
                  'ml-0.5 min-w-[18px] rounded-full px-1 py-px text-center text-[10px] font-semibold leading-tight',
                  activeTab === type.value
                    ? 'bg-primary/20 text-primary'
                    : 'bg-muted text-muted-foreground'
                )}>
                  {count}
                </span>
              )}
            </button>
          );
        })}

        {/* Remaining extra tabs (excluding files, already rendered above) */}
        {extrasWithoutFiles.length > 0 && (
          <div className="mx-0.5 h-5 w-px shrink-0 bg-border" />
        )}
        {extrasWithoutFiles.map((tab) => (
          <button
            key={tab.value}
            onClick={() => onTabChange(tab.value)}
            className={cn(
              'shrink-0 rounded-lg px-2.5 py-1.5 text-xs font-medium transition-all duration-150',
              activeTab === tab.value
                ? 'bg-primary/15 text-primary shadow-sm'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>
    </div>
  );
}
