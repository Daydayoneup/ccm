import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FolderOpen, Search, Download, Loader2, Terminal, Pin, PinOff, ChevronLeft, ChevronRight, AlertTriangle, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Input } from '@/components/ui/input';
import type { Project, DiscoveredProject } from '@/types/v2';

export function ProjectListPage() {
  const navigate = useNavigate();
  const {
    projects,
    discoveredProjects,
    claudeDiscoveredProjects,
    loading,
    error,
    loadProjects,
    registerProject,
    removeProject,
    discoverProjects,
    discoverFromClaude,
    togglePin,
  } = useProjectStoreV2();

  const [scanOpen, setScanOpen] = useState(false);
  const [scanDir, setScanDir] = useState('');
  const [scanning, setScanning] = useState(false);

  const [importOpen, setImportOpen] = useState(false);
  const [importLoading, setImportLoading] = useState(false);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [importProgress, setImportProgress] = useState<{ current: number; total: number } | null>(null);

  const [pendingDelete, setPendingDelete] = useState<Project | null>(null);
  const [deleteFromDisk, setDeleteFromDisk] = useState(false);

  const [searchQuery, setSearchQuery] = useState('');
  const [searchOpen, setSearchOpen] = useState(false);

  const filteredProjects = searchQuery
    ? projects.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          p.path.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : projects;

  const PAGE_SIZE = 12;
  const [currentPage, setCurrentPage] = useState(1);
  const totalPages = Math.max(1, Math.ceil(filteredProjects.length / PAGE_SIZE));
  const pagedProjects = filteredProjects.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  // Reset to page 1 when projects or search change
  useEffect(() => {
    setCurrentPage(1);
  }, [searchQuery]);

  useEffect(() => {
    if (currentPage > totalPages) setCurrentPage(totalPages);
  }, [filteredProjects.length, currentPage, totalPages]);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  const handleScan = async () => {
    if (!scanDir.trim()) return;
    setScanning(true);
    try {
      await discoverProjects([scanDir.trim()]);
    } finally {
      setScanning(false);
    }
  };

  const handleRegister = async (path: string) => {
    await registerProject(path);
  };

  const handleImportOpen = async () => {
    setImportOpen(true);
    setImportLoading(true);
    setSelectedPaths(new Set());
    setImportProgress(null);
    try {
      await discoverFromClaude();
    } finally {
      setImportLoading(false);
    }
  };

  const toggleSelection = (path: string) => {
    setSelectedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const selectAll = () => {
    setSelectedPaths(new Set(claudeDiscoveredProjects.map((p: DiscoveredProject) => p.path)));
  };

  const deselectAll = () => {
    setSelectedPaths(new Set());
  };

  const handleImportSelected = async () => {
    const paths = Array.from(selectedPaths);
    if (paths.length === 0) return;
    setImportProgress({ current: 0, total: paths.length });
    for (let i = 0; i < paths.length; i++) {
      setImportProgress({ current: i + 1, total: paths.length });
      try {
        await registerProject(paths[i]);
      } catch {
        // continue importing remaining projects
      }
    }
    setImportProgress(null);
    setImportOpen(false);
    await loadProjects();
  };

  return (
    <div className="space-y-6 p-8">
      <div className="flex items-center justify-between gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold tracking-tight">Projects</h1>
            {searchOpen ? (
              <div className="flex items-center gap-1.5 rounded-lg border bg-card px-3 py-1.5 shadow-sm transition-all">
                <Search className="size-3.5 shrink-0 text-muted-foreground" />
                <input
                  autoFocus
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="搜索项目名称或路径..."
                  className="w-48 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
                />
                {searchQuery && (
                  <span className="shrink-0 text-[10px] text-muted-foreground tabular-nums">
                    {filteredProjects.length}/{projects.length}
                  </span>
                )}
                <button
                  onClick={() => { setSearchOpen(false); setSearchQuery(''); }}
                  className="shrink-0 rounded p-0.5 text-muted-foreground hover:text-foreground"
                >
                  <X className="size-3.5" />
                </button>
              </div>
            ) : (
              <button
                onClick={() => setSearchOpen(true)}
                className="rounded-lg p-1.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                title="搜索项目"
              >
                <Search className="size-4" />
              </button>
            )}
          </div>
          <p className="mt-1 text-sm text-muted-foreground">
            Manage Claude Code configurations for your projects
          </p>
        </div>
        <div className="flex gap-2">
          <Button size="sm" variant="outline" className="rounded-lg" onClick={handleImportOpen}>
            <Download className="mr-1.5 size-3.5" />
            Import from Claude
          </Button>
          <Dialog open={scanOpen} onOpenChange={setScanOpen}>
            <DialogTrigger asChild>
              <Button size="sm" className="rounded-lg">
                <Search className="mr-1.5 size-3.5" />
                Scan for Projects
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Scan for Projects</DialogTitle>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="flex gap-2">
                  <Input
                    value={scanDir}
                    onChange={(e) => setScanDir(e.target.value)}
                    placeholder="/path/to/projects/directory"
                  />
                  <Button onClick={handleScan} disabled={scanning}>
                    {scanning ? 'Scanning...' : 'Scan'}
                  </Button>
                </div>
                {discoveredProjects.length > 0 && (
                  <div className="space-y-2">
                    <p className="text-sm font-medium">Found {discoveredProjects.length} project(s):</p>
                    {discoveredProjects.map((p) => (
                      <div key={p.path} className="flex items-center justify-between rounded-lg border p-2.5">
                        <div>
                          <div className="font-medium text-sm">{p.name}</div>
                          <div className="font-mono text-xs text-muted-foreground">{p.path}</div>
                        </div>
                        <Button size="sm" onClick={() => handleRegister(p.path)}>
                          Register
                        </Button>
                      </div>
                    ))}
                  </div>
                )}
                {discoveredProjects.length === 0 && scanDir && !scanning && (
                  <p className="text-sm text-muted-foreground">No new projects found.</p>
                )}
              </div>
            </DialogContent>
          </Dialog>
        </div>
      </div>

      <Dialog open={importOpen} onOpenChange={(open) => {
        if (!importProgress) setImportOpen(open);
      }}>
          <DialogContent className="flex max-h-[85vh] flex-col overflow-hidden sm:max-w-2xl p-0">
            {/* Sticky header */}
            <div className="flex items-center justify-between border-b px-5 py-4">
              <div>
                <DialogTitle className="text-base font-semibold">Import from Claude</DialogTitle>
                {!importLoading && claudeDiscoveredProjects.length > 0 && (
                  <p className="mt-0.5 text-xs text-muted-foreground">
                    {claudeDiscoveredProjects.length} projects found
                  </p>
                )}
              </div>
              {!importLoading && claudeDiscoveredProjects.length > 0 && (
                <label className="flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground">
                  <input
                    type="checkbox"
                    checked={selectedPaths.size === claudeDiscoveredProjects.length && claudeDiscoveredProjects.length > 0}
                    onChange={() => selectedPaths.size === claudeDiscoveredProjects.length ? deselectAll() : selectAll()}
                    className="size-3.5 rounded border-input accent-primary"
                  />
                  Select All
                </label>
              )}
            </div>

            {/* Scrollable list */}
            <div className="flex-1 overflow-y-auto px-2 py-1">
              {importLoading ? (
                <div className="flex items-center justify-center py-16 text-muted-foreground">
                  <Loader2 className="mr-2 size-5 animate-spin text-primary" />
                  Discovering projects...
                </div>
              ) : claudeDiscoveredProjects.length === 0 ? (
                <p className="py-16 text-center text-sm text-muted-foreground">
                  No projects found in ~/.claude/projects.
                </p>
              ) : (
                <div className="space-y-px">
                  {claudeDiscoveredProjects.map((p: DiscoveredProject) => (
                    <label
                      key={p.path}
                      className="flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 transition-colors hover:bg-accent/30"
                    >
                      <input
                        type="checkbox"
                        checked={selectedPaths.has(p.path)}
                        onChange={() => toggleSelection(p.path)}
                        className="size-3.5 shrink-0 rounded border-input accent-primary"
                      />
                      <span className="shrink-0 text-sm font-medium">{p.name}</span>
                      {p.has_claude_config && (
                        <span className="shrink-0 rounded bg-primary/10 px-1.5 py-px text-[10px] font-medium text-primary">.claude</span>
                      )}
                      <span className="min-w-0 flex-1 truncate text-right font-mono text-[11px] text-muted-foreground/60">
                        {p.path}
                      </span>
                    </label>
                  ))}
                </div>
              )}
            </div>

            {/* Sticky footer — always visible */}
            {!importLoading && claudeDiscoveredProjects.length > 0 && (
              <div className="flex items-center justify-between border-t bg-muted/30 px-5 py-3">
                <div className="text-xs text-muted-foreground tabular-nums">
                  {importProgress ? (
                    <span className="flex items-center gap-1.5">
                      <Loader2 className="size-3 animate-spin text-primary" />
                      Importing {importProgress.current}/{importProgress.total}...
                    </span>
                  ) : (
                    <span>
                      <span className="font-semibold text-foreground">{selectedPaths.size}</span>
                      {' '}of {claudeDiscoveredProjects.length} selected
                    </span>
                  )}
                </div>
                <Button
                  size="sm"
                  className="rounded-lg"
                  onClick={handleImportSelected}
                  disabled={selectedPaths.size === 0 || importProgress !== null}
                >
                  {importProgress ? 'Importing...' : `Import (${selectedPaths.size})`}
                </Button>
              </div>
            )}
          </DialogContent>
        </Dialog>

      {error && (
        <div className="rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-16 text-muted-foreground">
          Loading...
        </div>
      ) : projects.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-xl border border-dashed py-16 text-muted-foreground">
          <FolderOpen className="mb-4 size-12 opacity-40" />
          <p>No projects registered yet.</p>
          <p className="text-sm">Use "Scan for Projects" to discover projects.</p>
        </div>
      ) : filteredProjects.length === 0 && searchQuery ? (
        <div className="flex flex-col items-center justify-center rounded-xl border border-dashed py-16 text-muted-foreground">
          <Search className="mb-4 size-10 opacity-30" />
          <p>没有匹配 &quot;{searchQuery}&quot; 的项目</p>
        </div>
      ) : (<>
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          {pagedProjects.map((project) => (
            <div
              key={project.id}
              className="card-glow group cursor-pointer rounded-xl border bg-card p-4 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/5"
              onClick={() => navigate(`/projects/${project.id}`)}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="truncate font-semibold text-sm" title={project.name}>
                      {project.name}
                    </h3>
                    {project.language && (
                      <Badge variant="secondary" className="shrink-0 text-[10px]">
                        {project.language}
                      </Badge>
                    )}
                  </div>
                  <p className="mt-1.5 truncate font-mono text-xs text-muted-foreground" title={project.path}>
                    {project.path}
                  </p>
                </div>
              </div>
              <div className="mt-3 flex items-center justify-between gap-2 border-t border-border/50 pt-3">
                <div className="flex items-center gap-1">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      togglePin(project.id);
                    }}
                    className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                    title={project.pinned === 1 ? 'Unpin' : 'Pin'}
                  >
                    {project.pinned === 1 ? <PinOff className="size-4" /> : <Pin className="size-4" />}
                  </button>
                  <Button
                    size="sm"
                    className="gap-1.5 rounded-lg bg-primary/15 text-primary shadow-none hover:bg-primary hover:text-primary-foreground transition-all duration-200"
                    onClick={(e) => {
                      e.stopPropagation();
                      invoke('launch_claude_in_terminal', { projectPath: project.path, projectId: project.id });
                    }}
                  >
                    <Terminal className="size-3.5" />
                    启动 Shell
                  </Button>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs text-muted-foreground/50 hover:text-destructive"
                  onClick={(e) => {
                    e.stopPropagation();
                    setPendingDelete(project);
                    setDeleteFromDisk(false);
                  }}
                >
                  删除
                </Button>
              </div>
            </div>
          ))}
        </div>
        {totalPages > 1 && (
          <div className="flex items-center justify-center gap-2 pt-4">
            <Button
              variant="outline"
              size="sm"
              className="gap-1 rounded-lg"
              disabled={currentPage === 1}
              onClick={() => setCurrentPage((p) => p - 1)}
            >
              <ChevronLeft className="size-3.5" />
              Previous
            </Button>
            <span className="px-3 text-sm text-muted-foreground tabular-nums">
              {currentPage} / {totalPages}
            </span>
            <Button
              variant="outline"
              size="sm"
              className="gap-1 rounded-lg"
              disabled={currentPage === totalPages}
              onClick={() => setCurrentPage((p) => p + 1)}
            >
              Next
              <ChevronRight className="size-3.5" />
            </Button>
          </div>
        )}
      </>
      )}
      {/* Delete confirmation dialog */}
      <AlertDialog open={!!pendingDelete} onOpenChange={(open) => { if (!open) setPendingDelete(null); }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>确认删除项目</AlertDialogTitle>
            <AlertDialogDescription asChild>
              <div className="space-y-3">
                <p>
                  确定要从 CCM 中移除项目 <span className="font-semibold text-foreground">{pendingDelete?.name}</span> 吗？
                </p>
                <p className="truncate font-mono text-xs text-muted-foreground">
                  {pendingDelete?.path}
                </p>
                <label className="flex cursor-pointer items-center gap-2 rounded-lg border p-3 transition-colors hover:bg-muted">
                  <input
                    type="checkbox"
                    checked={deleteFromDisk}
                    onChange={(e) => setDeleteFromDisk(e.target.checked)}
                    className="size-4 rounded border-input accent-destructive"
                  />
                  <span className="text-sm font-medium">同时删除磁盘文件（不可恢复！）</span>
                </label>
                {deleteFromDisk && (
                  <div className="flex gap-2 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                    <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                    <div>
                      <p className="font-medium">警告：此操作无法撤销</p>
                      <p className="mt-1 text-xs leading-relaxed text-destructive/80">
                        将永久删除项目目录及其所有文件（包括源代码和 .claude/ 配置），
                        清理 ~/.claude/settings.json 中的项目配置，
                        并删除 ~/.claude/projects/ 下的会话数据。
                      </p>
                    </div>
                  </div>
                )}
              </div>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>取消</AlertDialogCancel>
            <AlertDialogAction
              className={deleteFromDisk ? 'bg-destructive text-destructive-foreground hover:bg-destructive/90' : ''}
              onClick={async () => {
                if (pendingDelete) {
                  await removeProject(pendingDelete.id, deleteFromDisk);
                  setPendingDelete(null);
                }
              }}
            >
              确认删除
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
