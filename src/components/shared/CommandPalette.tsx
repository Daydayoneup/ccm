import { useState, useEffect, useRef, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { Terminal, Search, Pin, PinOff, Loader2, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import type { Project } from '@/types/v2';

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
}

export function CommandPalette({ open, onClose }: CommandPaletteProps) {
  const navigate = useNavigate();
  const { togglePin } = useProjectStoreV2();
  const [query, setQuery] = useState('');
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    setQuery('');
    setSelectedIndex(0);
    setError(null);
    setLoading(true);
    invoke<Project[]>('list_projects_ranked')
      .then(setProjects)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
    setTimeout(() => inputRef.current?.focus(), 50);
  }, [open]);

  const filtered = query
    ? projects.filter(
        (p) =>
          p.name.toLowerCase().includes(query.toLowerCase()) ||
          p.path.toLowerCase().includes(query.toLowerCase())
      )
    : projects;

  const pinned = filtered.filter((p) => p.pinned === 1);
  const recent = filtered.filter((p) => p.pinned !== 1);
  const flatList = [...pinned, ...recent];

  useEffect(() => {
    if (selectedIndex >= flatList.length) {
      setSelectedIndex(Math.max(0, flatList.length - 1));
    }
  }, [flatList.length, selectedIndex]);

  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-index="${selectedIndex}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  const launchShell = useCallback(
    async (project: Project) => {
      try {
        await invoke('launch_claude_in_terminal', {
          projectPath: project.path,
          projectId: project.id,
        });
        onClose();
      } catch (e) {
        setError(String(e));
        setTimeout(() => setError(null), 3000);
      }
    },
    [onClose]
  );

  const navigateToProject = useCallback(
    (project: Project) => {
      navigate(`/projects/${project.id}`);
      onClose();
    },
    [navigate, onClose]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === 'ArrowDown' || (e.ctrlKey && e.key === 'j')) {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, flatList.length - 1));
        return;
      }
      if (e.key === 'ArrowUp' || (e.ctrlKey && e.key === 'k')) {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === 'Enter' && flatList[selectedIndex]) {
        e.preventDefault();
        if (e.metaKey) {
          navigateToProject(flatList[selectedIndex]);
        } else {
          launchShell(flatList[selectedIndex]);
        }
        return;
      }
    },
    [flatList, selectedIndex, onClose, launchShell, navigateToProject]
  );

  const handleTogglePin = useCallback(
    async (e: React.MouseEvent, projectId: string) => {
      e.stopPropagation();
      await togglePin(projectId);
      const updated = await invoke<Project[]>('list_projects_ranked');
      setProjects(updated);
    },
    [togglePin]
  );

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-[15vh] backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-xl border bg-popover shadow-2xl"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="flex items-center gap-2 border-b px-4 py-3">
          <Search className="size-4 shrink-0 text-muted-foreground" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            placeholder="搜索项目..."
            className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
          <kbd className="rounded border bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
            ESC
          </kbd>
        </div>

        <div ref={listRef} className="max-h-80 overflow-y-auto p-2">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="size-5 animate-spin text-muted-foreground" />
            </div>
          ) : flatList.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              {projects.length === 0 ? 'No projects registered' : 'No matching projects'}
            </p>
          ) : (
            <>
              {pinned.length > 0 && (
                <>
                  <p className="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                    Pinned
                  </p>
                  {pinned.map((project) => {
                    const idx = flatList.indexOf(project);
                    return (
                      <ProjectRow
                        key={project.id}
                        project={project}
                        isSelected={selectedIndex === idx}
                        dataIndex={idx}
                        onLaunch={() => launchShell(project)}
                        onNavigate={() => navigateToProject(project)}
                        onTogglePin={(e) => handleTogglePin(e, project.id)}
                      />
                    );
                  })}
                </>
              )}
              {recent.length > 0 && (
                <>
                  <p className="px-2 pb-1 pt-3 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                    Recent
                  </p>
                  {recent.map((project) => {
                    const idx = flatList.indexOf(project);
                    return (
                      <ProjectRow
                        key={project.id}
                        project={project}
                        isSelected={selectedIndex === idx}
                        dataIndex={idx}
                        onLaunch={() => launchShell(project)}
                        onNavigate={() => navigateToProject(project)}
                        onTogglePin={(e) => handleTogglePin(e, project.id)}
                      />
                    );
                  })}
                </>
              )}
            </>
          )}
        </div>

        {error && (
          <div className="flex items-center gap-2 border-t bg-destructive/10 px-4 py-2 text-xs text-destructive">
            <X className="size-3 shrink-0" />
            <span className="truncate">{error}</span>
          </div>
        )}

        <div className="flex items-center gap-3 border-t px-4 py-2 text-[10px] text-muted-foreground">
          <span><kbd className="rounded border bg-muted px-1">↵</kbd> Launch Shell</span>
          <span><kbd className="rounded border bg-muted px-1">⌘↵</kbd> Detail</span>
          <span><kbd className="rounded border bg-muted px-1">↑↓</kbd> Navigate</span>
        </div>
      </div>
    </div>
  );
}

interface ProjectRowProps {
  project: Project;
  isSelected: boolean;
  dataIndex: number;
  onLaunch: () => void;
  onNavigate: () => void;
  onTogglePin: (e: React.MouseEvent) => void;
}

function ProjectRow({ project, isSelected, dataIndex, onLaunch, onNavigate, onTogglePin }: ProjectRowProps) {
  const shortPath = project.path.replace(/^\/Users\/[^/]+/, '~');
  const maxLen = 35;
  const displayPath = shortPath.length > maxLen ? '...' + shortPath.slice(shortPath.length - maxLen) : shortPath;

  return (
    <div
      data-index={dataIndex}
      className={cn(
        'group flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 transition-colors',
        isSelected ? 'bg-accent text-accent-foreground' : 'hover:bg-muted'
      )}
      onClick={onLaunch}
      onDoubleClick={onNavigate}
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{project.name}</span>
          {project.language && (
            <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
              {project.language}
            </span>
          )}
        </div>
        <p className="truncate text-xs text-muted-foreground">{displayPath}</p>
      </div>
      <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
        <button
          onClick={onTogglePin}
          className="rounded p-1 hover:bg-background"
          title={project.pinned === 1 ? 'Unpin' : 'Pin'}
        >
          {project.pinned === 1 ? <PinOff className="size-3.5 text-muted-foreground" /> : <Pin className="size-3.5 text-muted-foreground" />}
        </button>
        <button
          onClick={(e) => { e.stopPropagation(); onLaunch(); }}
          className="rounded p-1 hover:bg-background"
          title="Launch Shell"
        >
          <Terminal className="size-3.5 text-primary" />
        </button>
      </div>
    </div>
  );
}
