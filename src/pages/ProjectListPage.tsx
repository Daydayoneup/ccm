import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import {
  AlertTriangle,
  ChevronLeft,
  ChevronRight,
  Download,
  FolderOpen,
  Loader2,
  Pin,
  PinOff,
  Search,
  Terminal,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
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
import { EmptyState, InlineStatus, PageHeader, PageShell, PanelSection, ToolbarRow } from '@/components/layout/PageShell';
import { useI18n } from '@/i18n/provider';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import type { DiscoveredProject, Project } from '@/types/v2';

const PAGE_SIZE = 12;

export function ProjectListPage() {
  const { t, formatNumber } = useI18n();
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
  const [currentPage, setCurrentPage] = useState(1);

  const filteredProjects = searchQuery
    ? projects.filter(
        (project) =>
          project.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          project.path.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : projects;

  const totalPages = Math.max(1, Math.ceil(filteredProjects.length / PAGE_SIZE));
  const pagedProjects = filteredProjects.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  useEffect(() => {
    setCurrentPage(1);
  }, [searchQuery]);

  useEffect(() => {
    if (currentPage > totalPages) {
      setCurrentPage(totalPages);
    }
  }, [currentPage, totalPages]);

  const handleScan = async () => {
    if (!scanDir.trim()) return;
    setScanning(true);
    try {
      await discoverProjects([scanDir.trim()]);
    } finally {
      setScanning(false);
    }
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

  const handleImportSelected = async () => {
    const paths = Array.from(selectedPaths);
    if (paths.length === 0) return;
    setImportProgress({ current: 0, total: paths.length });
    for (let index = 0; index < paths.length; index += 1) {
      setImportProgress({ current: index + 1, total: paths.length });
      try {
        await registerProject(paths[index]);
      } catch {
        // Continue importing remaining projects.
      }
    }
    setImportProgress(null);
    setImportOpen(false);
    await loadProjects();
  };

  return (
    <PageShell className="gap-5">
      <PageHeader
        eyebrow={t('nav.projects')}
        title={t('projects.title')}
        description={t('projects.subtitle')}
        actions={
          <>
            <Button size="sm" variant="outline" className="rounded-md" onClick={handleImportOpen}>
              <Download className="size-3.5" />
              {t('projects.importFromClaude')}
            </Button>
            <Dialog open={scanOpen} onOpenChange={setScanOpen}>
              <DialogTrigger asChild>
                <Button size="sm" className="rounded-md">
                  <Search className="size-3.5" />
                  {t('projects.scanForProjects')}
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>{t('projects.scanDialogTitle')}</DialogTitle>
                </DialogHeader>
                <div className="space-y-4 py-4">
                  <div className="flex gap-2">
                    <Input
                      value={scanDir}
                      onChange={(e) => setScanDir(e.target.value)}
                      placeholder={t('projects.scanDialogPlaceholder')}
                    />
                    <Button onClick={handleScan} disabled={scanning}>
                      {scanning ? t('projects.scanning') : t('projects.scanAction')}
                    </Button>
                  </div>
                  {discoveredProjects.length > 0 && (
                    <div className="space-y-2">
                      <p className="text-sm font-medium">
                        {t('projects.foundProjects', { count: formatNumber(discoveredProjects.length) })}
                      </p>
                      {discoveredProjects.map((project) => (
                        <div key={project.path} className="flex items-center justify-between rounded-md border p-3">
                          <div>
                            <div className="text-sm font-medium">{project.name}</div>
                            <div className="font-mono text-xs text-muted-foreground">{project.path}</div>
                          </div>
                          <Button size="sm" onClick={() => registerProject(project.path)}>
                            {t('projects.register')}
                          </Button>
                        </div>
                      ))}
                    </div>
                  )}
                  {discoveredProjects.length === 0 && scanDir && !scanning && (
                    <p className="text-sm text-muted-foreground">{t('projects.noNewProjects')}</p>
                  )}
                </div>
              </DialogContent>
            </Dialog>
          </>
        }
      />

      <ToolbarRow>
        <div className="relative w-full max-w-md">
          <Search className="absolute left-3 top-3.5 size-4 text-muted-foreground" />
          <Input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t('projects.searchPlaceholder')}
            className="h-11 rounded-md border-border/70 bg-panel pl-10"
          />
        </div>
        <div className="text-sm text-muted-foreground">
          {formatNumber(filteredProjects.length)} / {formatNumber(projects.length)}
        </div>
      </ToolbarRow>

      <Dialog open={importOpen} onOpenChange={(open) => { if (!importProgress) setImportOpen(open); }}>
        <DialogContent className="flex max-h-[85vh] flex-col overflow-hidden p-0 sm:max-w-2xl">
          <div className="flex items-center justify-between border-b px-5 py-4">
            <div>
              <DialogTitle className="text-base font-semibold">{t('projects.importDialogTitle')}</DialogTitle>
              {!importLoading && claudeDiscoveredProjects.length > 0 && (
                <p className="mt-1 text-xs text-muted-foreground">
                  {t('projects.discoveredProjects', { count: formatNumber(claudeDiscoveredProjects.length) })}
                </p>
              )}
            </div>
            {!importLoading && claudeDiscoveredProjects.length > 0 && (
              <label className="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground">
                <input
                  type="checkbox"
                  checked={selectedPaths.size === claudeDiscoveredProjects.length && claudeDiscoveredProjects.length > 0}
                  onChange={() =>
                    setSelectedPaths(
                      selectedPaths.size === claudeDiscoveredProjects.length
                        ? new Set()
                        : new Set(claudeDiscoveredProjects.map((project: DiscoveredProject) => project.path))
                    )
                  }
                  className="size-3.5 rounded border-input accent-primary"
                />
                {t('projects.selectAll')}
              </label>
            )}
          </div>

          <div className="flex-1 overflow-y-auto px-2 py-1">
            {importLoading ? (
              <div className="flex items-center justify-center py-16 text-muted-foreground">
                <Loader2 className="mr-2 size-5 animate-spin text-primary" />
                {t('projects.discovering')}
              </div>
            ) : claudeDiscoveredProjects.length === 0 ? (
              <p className="py-16 text-center text-sm text-muted-foreground">
                {t('projects.noClaudeProjects')}
              </p>
            ) : (
              <div className="space-y-px">
                {claudeDiscoveredProjects.map((project) => (
                  <label
                    key={project.path}
                    className="flex cursor-pointer items-center gap-3 rounded-md px-3 py-2 transition-colors hover:bg-accent/30"
                  >
                    <input
                      type="checkbox"
                      checked={selectedPaths.has(project.path)}
                      onChange={() => {
                        setSelectedPaths((prev) => {
                          const next = new Set(prev);
                          if (next.has(project.path)) next.delete(project.path);
                          else next.add(project.path);
                          return next;
                        });
                      }}
                      className="size-3.5 shrink-0 rounded border-input accent-primary"
                    />
                    <span className="shrink-0 text-sm font-medium">{project.name}</span>
                    {project.has_claude_config ? (
                      <span className="shrink-0 rounded bg-primary/10 px-1.5 py-px text-[10px] font-medium text-primary">.claude</span>
                    ) : null}
                    <span className="min-w-0 flex-1 truncate text-right font-mono text-[11px] text-muted-foreground/60">
                      {project.path}
                    </span>
                  </label>
                ))}
              </div>
            )}
          </div>

          {!importLoading && claudeDiscoveredProjects.length > 0 && (
            <div className="flex items-center justify-between border-t bg-muted/30 px-5 py-3">
              <div className="text-xs text-muted-foreground tabular-nums">
                {importProgress ? (
                  <span className="flex items-center gap-1.5">
                    <Loader2 className="size-3 animate-spin text-primary" />
                    {t('projects.importingProgress', importProgress)}
                  </span>
                ) : (
                  t('common.selectedCount', {
                    selected: formatNumber(selectedPaths.size),
                    total: formatNumber(claudeDiscoveredProjects.length),
                  })
                )}
              </div>
              <Button
                size="sm"
                className="rounded-md"
                onClick={handleImportSelected}
                disabled={selectedPaths.size === 0 || importProgress !== null}
              >
                {importProgress ? t('projects.importing') : t('projects.importSelected', { count: formatNumber(selectedPaths.size) })}
              </Button>
            </div>
          )}
        </DialogContent>
      </Dialog>

      {error ? <InlineStatus tone="danger">{error}</InlineStatus> : null}

      <PanelSection>
        {loading ? (
          <div className="flex items-center justify-center py-16 text-muted-foreground">
            {t('projects.loading')}
          </div>
        ) : projects.length === 0 ? (
          <EmptyState
            icon={<FolderOpen className="size-10 opacity-50" />}
            title={t('projects.noProjects')}
            description={t('projects.noProjectsHint')}
          />
        ) : filteredProjects.length === 0 ? (
          <EmptyState
            icon={<Search className="size-10 opacity-50" />}
            title={t('projects.noSearchResults', { query: searchQuery })}
          />
        ) : (
          <>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
              {pagedProjects.map((project) => (
                <div
                  key={project.id}
                  className="card-glow group cursor-pointer rounded-md border bg-card/90 p-5 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-[0_20px_50px_rgba(15,23,42,0.10)]"
                  onClick={() => navigate(`/projects/${project.id}`)}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <h3 className="truncate text-sm font-semibold" title={project.name}>
                          {project.name}
                        </h3>
                        {project.language ? <Badge variant="secondary" className="shrink-0 text-[10px]">{project.language}</Badge> : null}
                      </div>
                      <p className="mt-3 truncate text-xs font-mono text-muted-foreground" title={project.path}>
                        {project.path}
                      </p>
                      <p className="mt-2 text-xs text-muted-foreground">
                        {t('projects.recentLaunches', { count: formatNumber(project.launch_count) })}
                      </p>
                    </div>
                  </div>
                  <div className="mt-4 flex items-center justify-between gap-2 border-t border-border/50 pt-3">
                    <div className="flex items-center gap-1">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          togglePin(project.id);
                        }}
                        className="rounded-sm p-2 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                        title={project.pinned === 1 ? t('projects.unpin') : t('projects.pin')}
                      >
                        {project.pinned === 1 ? <PinOff className="size-4" /> : <Pin className="size-4" />}
                      </button>
                      <Button
                        size="sm"
                        className="gap-1.5 rounded-md bg-primary/15 text-primary shadow-none hover:bg-primary hover:text-primary-foreground"
                        onClick={(e) => {
                          e.stopPropagation();
                          invoke('launch_claude_in_terminal', { projectPath: project.path, projectId: project.id });
                        }}
                      >
                        <Terminal className="size-3.5" />
                        {t('common.launchShell')}
                      </Button>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-xs text-muted-foreground/60 hover:text-destructive"
                      onClick={(e) => {
                        e.stopPropagation();
                        setPendingDelete(project);
                        setDeleteFromDisk(false);
                      }}
                    >
                      {t('common.delete')}
                    </Button>
                  </div>
                </div>
              ))}
            </div>

            {totalPages > 1 ? (
              <div className="flex items-center justify-center gap-2 pt-4">
                <Button
                  variant="outline"
                  size="sm"
                  className="gap-1 rounded-md"
                  disabled={currentPage === 1}
                  onClick={() => setCurrentPage((page) => page - 1)}
                >
                  <ChevronLeft className="size-3.5" />
                  {t('projects.previous')}
                </Button>
                <span className="px-3 text-sm tabular-nums text-muted-foreground">
                  {t('projects.page', { current: currentPage, total: totalPages })}
                </span>
                <Button
                  variant="outline"
                  size="sm"
                  className="gap-1 rounded-md"
                  disabled={currentPage === totalPages}
                  onClick={() => setCurrentPage((page) => page + 1)}
                >
                  {t('projects.next')}
                  <ChevronRight className="size-3.5" />
                </Button>
              </div>
            ) : null}
          </>
        )}
      </PanelSection>

      <AlertDialog open={!!pendingDelete} onOpenChange={(open) => { if (!open) setPendingDelete(null); }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('projects.deleteTitle')}</AlertDialogTitle>
            <AlertDialogDescription asChild>
              <div className="space-y-3">
                <p>{pendingDelete ? t('projects.deletePrompt', { name: pendingDelete.name }) : ''}</p>
                <p className="truncate font-mono text-xs text-muted-foreground">{pendingDelete?.path}</p>
                <label className="flex cursor-pointer items-center gap-2 rounded-lg border p-3 transition-colors hover:bg-muted">
                  <input
                    type="checkbox"
                    checked={deleteFromDisk}
                    onChange={(e) => setDeleteFromDisk(e.target.checked)}
                    className="size-4 rounded border-input accent-destructive"
                  />
                  <span className="text-sm font-medium">{t('projects.deleteFromDisk')}</span>
                </label>
                {deleteFromDisk ? (
                  <div className="flex gap-2 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                    <AlertTriangle className="mt-0.5 size-4 shrink-0" />
                    <div>
                      <p className="font-medium">{t('projects.deleteWarningTitle')}</p>
                      <p className="mt-1 text-xs leading-relaxed text-destructive/80">{t('projects.deleteWarningBody')}</p>
                    </div>
                  </div>
                ) : null}
              </div>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              className={deleteFromDisk ? 'bg-destructive text-destructive-foreground hover:bg-destructive/90' : ''}
              onClick={async () => {
                if (pendingDelete) {
                  await removeProject(pendingDelete.id, deleteFromDisk);
                  setPendingDelete(null);
                }
              }}
            >
              {t('common.confirmDelete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </PageShell>
  );
}
