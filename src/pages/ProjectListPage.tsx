import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { launchClaudeInTerminal } from '@/lib/tauri-api';
import {
  ChevronLeft,
  ChevronRight,
  Download,
  FolderOpen,
  Search,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { EmptyState, InlineStatus, PageHeader, PageShell, PanelSection, ToolbarRow } from '@/components/layout/PageShell';
import { useI18n } from '@/i18n/provider';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import type { Project } from '@/types/v2';
import { ScanDialog } from './project-list/ScanDialog';
import { ImportDialog } from './project-list/ImportDialog';
import { DeleteConfirmDialog } from './project-list/DeleteConfirmDialog';
import { ProjectCard } from './project-list/ProjectCard';

const PAGE_SIZE = 12;

export function ProjectListPage() {
  const { t, formatNumber } = useI18n();
  const navigate = useNavigate();
  const {
    projects,
    loading,
    error,
    loadProjects,
    removeProject,
    togglePin,
  } = useProjectStoreV2();

  const [scanOpen, setScanOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<Project | null>(null);
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

  return (
    <PageShell className="gap-5">
      <PageHeader
        eyebrow={t('nav.projects')}
        title={t('projects.title')}
        description={t('projects.subtitle')}
        actions={
          <>
            <Button size="sm" variant="outline" className="rounded-md" onClick={() => setImportOpen(true)}>
              <Download className="size-3.5" />
              {t('projects.importFromClaude')}
            </Button>
            <Button size="sm" className="rounded-md" onClick={() => setScanOpen(true)}>
              <Search className="size-3.5" />
              {t('projects.scanForProjects')}
            </Button>
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

      <ScanDialog open={scanOpen} onOpenChange={setScanOpen} onRegistered={() => loadProjects()} />
      <ImportDialog open={importOpen} onOpenChange={setImportOpen} onImported={() => loadProjects()} />

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
                <ProjectCard
                  key={project.id}
                  project={project}
                  onNavigate={() => navigate(`/projects/${project.id}`)}
                  onDelete={() => setPendingDelete(project)}
                  onTogglePin={() => togglePin(project.id)}
                  onLaunch={() => launchClaudeInTerminal(project.path)}
                />
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

      <DeleteConfirmDialog
        project={pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={async (id, fromDisk) => {
          await removeProject(id, fromDisk);
          setPendingDelete(null);
        }}
      />
    </PageShell>
  );
}
