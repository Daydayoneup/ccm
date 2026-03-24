import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { ScopeBadge } from '@/lib/scope-utils';
import type { Resource } from '@/types/v2';
import { useI18n } from '@/i18n/provider';

interface GlobalSearchProps {
  onSearch: (query: string) => void;
  results: Resource[];
  query: string;
}

export function GlobalSearch({ onSearch, results, query }: GlobalSearchProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();

  function handleChange(value: string) {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      onSearch(value);
    }, 300);
  }

  useEffect(() => {
    setOpen(query.length > 0 && results.length > 0);
  }, [query, results]);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <div ref={containerRef} className="relative w-full max-w-md">
      <div className="relative">
        <Search className="absolute left-3 top-3.5 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder={t('dashboard.searchPlaceholder')}
          className="h-11 rounded-md border-border/70 bg-panel pl-10 shadow-sm"
          defaultValue={query}
          onChange={(e) => handleChange(e.target.value)}
          onFocus={() => {
            if (query.length > 0 && results.length > 0) setOpen(true);
          }}
        />
      </div>
      {open && (
        <div className="absolute top-full z-50 mt-2 max-h-72 w-full overflow-y-auto rounded-md border border-border/70 bg-popover shadow-2xl">
          {results.map((resource) => (
            <div
              key={resource.id}
              className="flex cursor-pointer items-center justify-between gap-3 px-3 py-2 text-sm hover:bg-accent"
              onClick={() => {
                setOpen(false);
                const filePath = resource.source_path;
                const extra = resource.resource_type === 'skill'
                  ? `&resource_id=${resource.id}&type=skill&scope=${resource.scope === 'project' ? 'project' : 'library'}`
                  : '';
                navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
              }}
            >
              <span className="mr-2 truncate">{resource.name}</span>
              <div className="flex shrink-0 gap-1">
                <Badge variant="outline" className="text-xs">
                  {t(`resourceTypes.${resource.resource_type}`)}
                </Badge>
                <ScopeBadge scope={resource.scope} className="text-xs" />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
