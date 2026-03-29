import { Pin, PinOff, Terminal } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useI18n } from '@/i18n/provider';
import type { Project } from '@/types/v2';

interface ProjectCardProps {
  project: Project;
  onNavigate: () => void;
  onDelete: () => void;
  onTogglePin: () => void;
  onLaunch: () => void;
}

export function ProjectCard({ project, onNavigate, onDelete, onTogglePin, onLaunch }: ProjectCardProps) {
  const { t, formatNumber } = useI18n();

  return (
    <div
      className="card-glow group cursor-pointer rounded-md border bg-card/90 p-5 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-[0_20px_50px_rgba(15,23,42,0.10)]"
      onClick={onNavigate}
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
            onClick={(e) => { e.stopPropagation(); onTogglePin(); }}
            className="rounded-sm p-2 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
            title={project.pinned === 1 ? t('projects.unpin') : t('projects.pin')}
          >
            {project.pinned === 1 ? <PinOff className="size-4" /> : <Pin className="size-4" />}
          </button>
          <Button
            size="sm"
            className="gap-1.5 rounded-md bg-primary/15 text-primary shadow-none hover:bg-primary hover:text-primary-foreground"
            onClick={(e) => { e.stopPropagation(); onLaunch(); }}
          >
            <Terminal className="size-3.5" />
            {t('common.launchShell')}
          </Button>
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="text-xs text-muted-foreground/60 hover:text-destructive"
          onClick={(e) => { e.stopPropagation(); onDelete(); }}
        >
          {t('common.delete')}
        </Button>
      </div>
    </div>
  );
}
