import { useEffect, useState } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import {
  LayoutDashboard,
  Globe,
  FolderGit2,
  Library,
  Settings,
  Terminal,
  Command,
  ChevronRight,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useProjectStoreV2 } from '@/stores/project-store-v2';
import { useI18n } from '@/i18n/provider';

const navItems = [
  { to: '/', key: 'dashboard', icon: LayoutDashboard },
  { to: '/projects', key: 'projects', icon: FolderGit2 },
  { to: '/global', key: 'global', icon: Globe },
  { to: '/library', key: 'library', icon: Library },
  { to: '/settings', key: 'settings', icon: Settings },
];

interface SidebarProps {
  paletteEnabled: boolean;
  onOpenPalette?: () => void;
}

export function Sidebar({ paletteEnabled }: SidebarProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { projects, loadProjects } = useProjectStoreV2();
  const [collapsed, setCollapsed] = useState(false);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  const pinnedProjects = projects
    .filter((p) => p.pinned === 1)
    .sort((a, b) => b.launch_count - a.launch_count);

  const handleLaunch = async (projectId: string, projectPath: string) => {
    try {
      await invoke('launch_claude_in_terminal', { projectPath, projectId });
    } catch (error) {
      console.error('Failed to launch shell:', error);
    }
  };

  return (
    <aside className={cn(
      "flex h-screen flex-col border-r border-sidebar-border bg-sidebar/95 py-4 text-sidebar-foreground backdrop-blur transition-[width] duration-200",
      collapsed ? "w-16 px-2" : "w-[288px] px-4"
    )}>
      {/* Header */}
      {collapsed ? (
        <div className="flex flex-col items-center gap-2">
          <div className="flex size-11 items-center justify-center rounded-md bg-primary/18 ring-1 ring-primary/20">
            <img src="/app-icon.png" alt="CCM" className="size-5" />
          </div>
          <button
            onClick={() => setCollapsed(false)}
            className="rounded-md p-1.5 text-sidebar-foreground/50 hover:bg-sidebar-accent hover:text-sidebar-foreground"
            title="展开侧边栏"
          >
            <PanelLeftOpen className="size-4" />
          </button>
        </div>
      ) : (
        <div className="rounded-lg border border-sidebar-border/80 bg-sidebar-accent/60 p-4">
          <div className="flex items-center gap-3">
            <div className="flex size-11 items-center justify-center rounded-md bg-primary/18 ring-1 ring-primary/20">
              <img src="/app-icon.png" alt="CCM" className="size-5" />
            </div>
            <div className="min-w-0 flex-1">
              <h1 className="text-sm font-semibold tracking-tight text-sidebar-accent-foreground">{t('common.appName')}</h1>
              <p className="text-[11px] leading-none text-sidebar-foreground/55">{t('common.appSubtitle')}</p>
            </div>
            <button
              onClick={() => setCollapsed(true)}
              className="shrink-0 rounded-md p-1.5 text-sidebar-foreground/40 hover:bg-sidebar-accent hover:text-sidebar-foreground"
              title="折叠侧边栏"
            >
              <PanelLeftClose className="size-4" />
            </button>
          </div>
          <div className="mt-4 flex items-center gap-2 rounded-md border border-sidebar-border/70 bg-sidebar/60 px-3 py-2 text-xs text-sidebar-foreground/70">
            <Command className="size-3.5 text-sidebar-primary" />
            <span>{paletteEnabled ? t('nav.pinHint') : t('nav.noPinnedProjects')}</span>
          </div>
        </div>
      )}

      {/* Navigation */}
      <nav className={cn("mt-5 flex flex-col gap-1", collapsed && "items-center")}>
        {!collapsed && (
          <span className="mb-1 px-3 text-[10px] font-semibold uppercase tracking-[0.22em] text-sidebar-foreground/45">
            {t('nav.navigation')}
          </span>
        )}
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === '/'}
            title={collapsed ? t(`nav.${item.key}`) : undefined}
            className={({ isActive }) =>
              cn(
                'group flex items-center rounded-md text-sm font-medium transition-all duration-200',
                collapsed ? 'justify-center p-2.5' : 'gap-3 px-3.5 py-3',
                isActive
                  ? 'bg-sidebar-primary/16 text-sidebar-primary shadow-[inset_3px_0_0_0] shadow-sidebar-primary'
                  : 'text-sidebar-foreground/72 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'
              )
            }
          >
            <item.icon className="size-[18px] shrink-0" />
            {!collapsed && <span className="flex-1">{t(`nav.${item.key}`)}</span>}
            {!collapsed && <ChevronRight className="size-4 shrink-0 opacity-0 transition-opacity group-hover:opacity-50" />}
          </NavLink>
        ))}
      </nav>

      {/* Pinned projects */}
      {!collapsed ? (
        <div className="mt-5 min-h-0 flex-1 rounded-lg border border-sidebar-border/70 bg-sidebar-accent/40 px-3 py-3">
          <span className="mb-2 block px-2 text-[10px] font-semibold uppercase tracking-[0.22em] text-sidebar-foreground/45">
            {t('nav.pinnedProjects')}
          </span>
          <div className="max-h-full space-y-1 overflow-y-auto scrollbar-none">
            {pinnedProjects.length === 0 ? (
              <p className="px-2 py-3 text-[11px] leading-5 text-sidebar-foreground/35">
                {paletteEnabled ? t('nav.pinHint') : t('nav.noPinnedProjects')}
              </p>
            ) : (
              pinnedProjects.map((project) => (
                <div
                  key={project.id}
                  className="group flex items-center gap-2 rounded-md px-2 py-2 transition-colors hover:bg-sidebar-accent"
                >
                  <button
                    onClick={() => navigate(`/projects/${project.id}`)}
                    className="min-w-0 flex-1 text-left"
                    title={project.path}
                  >
                    <div className="truncate text-[12px] font-medium text-sidebar-foreground/88">
                      {project.name}
                    </div>
                    <div className="truncate text-[10px] text-sidebar-foreground/45">
                      {project.path}
                    </div>
                  </button>
                  <button
                    onClick={() => handleLaunch(project.id, project.path)}
                    className="shrink-0 rounded-sm p-2 opacity-0 transition-opacity group-hover:opacity-100 hover:bg-sidebar-border"
                    title={t('common.launchShell')}
                  >
                    <Terminal className="size-3.5 text-sidebar-primary" />
                  </button>
                </div>
              ))
            )}
          </div>
        </div>
      ) : (
        <div className="mt-5 flex-1" />
      )}

      {!collapsed && (
        <div className="mt-4 px-2">
          <p className="text-center text-[10px] text-sidebar-foreground/32">{t('nav.version', { version: '2.0' })}</p>
        </div>
      )}
    </aside>
  );
}
