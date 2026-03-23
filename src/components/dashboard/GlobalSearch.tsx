import { useEffect, useRef, useState } from 'react';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Search } from 'lucide-react';
import { ScopeBadge } from '@/lib/scope-utils';
import type { Resource } from '@/types/v2';
import { useNavigate } from 'react-router-dom';

interface GlobalSearchProps {
  onSearch: (query: string) => void;
  results: Resource[];
  query: string;
}

export function GlobalSearch({ onSearch, results, query }: GlobalSearchProps) {
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
    <div ref={containerRef} className="relative w-72">
      <div className="relative">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder="Search resources..."
          className="pl-9"
          defaultValue={query}
          onChange={(e) => handleChange(e.target.value)}
          onFocus={() => {
            if (query.length > 0 && results.length > 0) setOpen(true);
          }}
        />
      </div>
      {open && (
        <div className="absolute top-full mt-1 w-full rounded-md border bg-popover shadow-lg z-50 max-h-64 overflow-y-auto">
          {results.map((resource) => (
            <div
              key={resource.id}
              className="flex items-center justify-between px-3 py-2 text-sm cursor-pointer hover:bg-accent"
              onClick={() => {
                setOpen(false);
                const filePath = resource.resource_type === 'skill'
                  ? `${resource.source_path}/SKILL.md`
                  : resource.source_path;
                const extra = resource.resource_type === 'skill'
                  ? `&resource_id=${resource.id}&type=skill`
                  : '';
                navigate(`/editor?file=${encodeURIComponent(filePath)}${extra}`);
              }}
            >
              <span className="truncate mr-2">{resource.name}</span>
              <div className="flex gap-1 shrink-0">
                <Badge variant="outline" className="text-xs">
                  {resource.resource_type}
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
