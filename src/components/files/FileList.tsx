import { useState, useEffect, useCallback } from 'react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Loader2, Folder, File, ChevronRight, Plus } from 'lucide-react';
import { listDirectory, writeFile, createDirectory } from '@/lib/tauri-api';
import type { FileEntry } from '@/lib/tauri-api';

interface FileListProps {
  rootPath: string;
  onFileClick: (filePath: string) => void;
  onFileCreate?: (filePath: string) => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function FileList({ rootPath, onFileClick, onFileCreate }: FileListProps) {
  const [currentPath, setCurrentPath] = useState(rootPath);
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createType, setCreateType] = useState<'file' | 'folder'>('file');
  const [createName, setCreateName] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const loadDirectory = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await listDirectory(path);
      setEntries(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadDirectory(currentPath);
  }, [currentPath, loadDirectory]);

  // Reset to root when rootPath changes (different project)
  useEffect(() => {
    setCurrentPath(rootPath);
    setSearchQuery('');
  }, [rootPath]);

  const handleDirectoryClick = (path: string) => {
    setCurrentPath(path);
    setSearchQuery('');
  };

  const handleCreate = async () => {
    const name = createName.trim();
    if (!name) return;
    if (name.includes('/') || name.includes('\\')) {
      setCreateError('Name cannot contain / or \\');
      return;
    }
    setCreating(true);
    setCreateError(null);
    const fullPath = `${currentPath}/${name}`;
    try {
      if (createType === 'file') {
        await writeFile(fullPath, '');
        setCreateOpen(false);
        setCreateName('');
        onFileCreate?.(fullPath);
      } else {
        await createDirectory(fullPath);
        setCreateOpen(false);
        setCreateName('');
        loadDirectory(currentPath);
      }
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  };

  // Build breadcrumb segments from rootPath to currentPath
  const breadcrumbs = (() => {
    const segments: { label: string; path: string }[] = [{ label: 'root', path: rootPath }];
    if (currentPath !== rootPath && currentPath.startsWith(rootPath)) {
      const relative = currentPath.slice(rootPath.length).replace(/^\//, '');
      const parts = relative.split('/');
      let accumulated = rootPath;
      for (const part of parts) {
        accumulated = `${accumulated}/${part}`;
        segments.push({ label: part, path: accumulated });
      }
    }
    return segments;
  })();

  // Filter entries by search query (case-insensitive)
  const filteredEntries = searchQuery
    ? entries.filter((e) => e.name.toLowerCase().includes(searchQuery.toLowerCase()))
    : entries;

  // Sort: directories first, then files, alphabetically
  const sortedEntries = [...filteredEntries].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <div className="space-y-3">
      {/* Breadcrumb + Search */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-1 text-sm text-muted-foreground overflow-hidden">
          {breadcrumbs.map((crumb, i) => (
            <span key={crumb.path} className="flex items-center gap-1 shrink-0">
              {i > 0 && <ChevronRight className="size-3" />}
              <button
                onClick={() => handleDirectoryClick(crumb.path)}
                className="hover:text-foreground transition-colors truncate"
              >
                {crumb.label}
              </button>
            </span>
          ))}
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            className="size-8"
            onClick={() => {
              setCreateOpen(true);
              setCreateType('file');
              setCreateName('');
              setCreateError(null);
            }}
            title="New file or folder"
          >
            <Plus className="size-4" />
          </Button>
          <Input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search files..."
            className="w-48 h-8 text-sm"
          />
        </div>
      </div>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {createType === 'file' ? 'New File' : 'New Folder'}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="flex gap-2">
              <Button
                variant={createType === 'file' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setCreateType('file')}
              >
                <File className="size-4 mr-1" />
                File
              </Button>
              <Button
                variant={createType === 'folder' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setCreateType('folder')}
              >
                <Folder className="size-4 mr-1" />
                Folder
              </Button>
            </div>
            <Input
              value={createName}
              onChange={(e) => {
                setCreateName(e.target.value);
                setCreateError(null);
              }}
              placeholder={createType === 'file' ? 'filename.txt' : 'folder-name'}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && createName.trim()) handleCreate();
              }}
              autoFocus
            />
            {createError && (
              <div className="text-sm text-destructive">{createError}</div>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setCreateOpen(false)}>
                Cancel
              </Button>
              <Button onClick={handleCreate} disabled={!createName.trim() || creating}>
                {creating ? 'Creating...' : 'Create'}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Content */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="size-6 animate-spin text-muted-foreground" />
        </div>
      ) : error ? (
        <div className="rounded-md border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : sortedEntries.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground text-sm">
          {searchQuery ? 'No files match your search.' : 'Empty directory.'}
        </div>
      ) : (
        <div className="rounded-md border">
          {sortedEntries.map((entry) => (
            <button
              key={entry.path}
              onClick={() =>
                entry.is_dir ? handleDirectoryClick(entry.path) : onFileClick(entry.path)
              }
              className="flex w-full items-center gap-3 px-3 py-2 text-sm hover:bg-muted/50 transition-colors border-b last:border-b-0 text-left"
            >
              {entry.is_dir ? (
                <Folder className="size-4 text-blue-400 shrink-0" />
              ) : (
                <File className="size-4 text-muted-foreground shrink-0" />
              )}
              <span className="flex-1 truncate">
                {entry.name}
                {entry.is_dir && '/'}
              </span>
              {entry.is_symlink && (
                <span className="text-xs text-muted-foreground">symlink</span>
              )}
              {!entry.is_dir && (
                <span className="text-xs text-muted-foreground shrink-0">
                  {formatSize(entry.size)}
                </span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
