import { useEffect } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import {
  LayoutDashboard, Globe, FolderGit2, Library, Settings, Search, Terminal,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useProjectStoreV2 } from '@/stores/project-store-v2';

const navItems = [
  { to: '/', label: '仪表盘', icon: LayoutDashboard },
  { to: '/projects', label: '项目列表', icon: FolderGit2 },
  { to: '/global', label: '全局资源', icon: Globe },
  { to: '/library', label: '资源库', icon: Library },
  { to: '/settings', label: '设置', icon: Settings },
];

interface SidebarProps {
  paletteEnabled: boolean;
  onOpenPalette: () => void;
}

export function Sidebar({ paletteEnabled, onOpenPalette }: SidebarProps) {
  const navigate = useNavigate();
  const { projects, loadProjects } = useProjectStoreV2();

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  const pinnedProjects = projects
    .filter((p) => p.pinned === 1)
    .sort((a, b) => b.launch_count - a.launch_count);

  const handleLaunch = async (projectId: string, projectPath: string) => {
    try {
      await invoke('launch_claude_in_terminal', { projectPath, projectId });
    } catch (e) {
      console.error('Failed to launch shell:', e);
    }
  };

  return (
    <aside className="flex h-screen w-56 flex-col bg-sidebar text-sidebar-foreground">
      <div className="flex h-14 items-center justify-between border-b border-sidebar-border px-4">
        <div className="flex items-center gap-2.5">
          <div className="flex size-8 items-center justify-center rounded-lg bg-primary/20">
            <img src="/app-icon.png" alt="CCM" className="size-5" />
          </div>
          <div>
            <h1 className="text-sm font-semibold tracking-tight text-sidebar-accent-foreground">CCM</h1>
            <p className="text-[10px] leading-none text-sidebar-foreground/50">Config Manager</p>
          </div>
        </div>
        {paletteEnabled && (
          <button
            onClick={onOpenPalette}
            className="rounded-md p-1.5 text-sidebar-foreground/50 transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
            title="Quick Launch"
          >
            <Search className="size-4" />
          </button>
        )}
      </div>

      <nav className="flex flex-1 flex-col gap-0.5 p-3">
        <span className="mb-2 px-3 text-[10px] font-semibold uppercase tracking-widest text-sidebar-foreground/40">
          Navigation
        </span>
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === '/'}
            className={({ isActive }) =>
              cn(
                'group flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-all duration-200',
                isActive
                  ? 'bg-sidebar-primary/15 text-sidebar-primary shadow-[inset_3px_0_0_0] shadow-sidebar-primary'
                  : 'text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'
              )
            }
          >
            <item.icon className="size-[18px] shrink-0" />
            {item.label}
          </NavLink>
        ))}
      </nav>

      <div className="mx-3 border-t border-sidebar-border pt-3">
        <span className="mb-2 block px-3 text-[10px] font-semibold uppercase tracking-widest text-sidebar-foreground/40">
          Pinned Projects
        </span>
        <div className="max-h-40 space-y-0.5 overflow-y-auto scrollbar-none">
          {pinnedProjects.length === 0 ? (
            <p className="px-3 py-2 text-[11px] text-sidebar-foreground/30">
              {paletteEnabled ? '⌘K to pin projects' : 'No pinned projects'}
            </p>
          ) : (
            pinnedProjects.map((project) => (
              <div
                key={project.id}
                className="group flex items-center gap-2 rounded-lg px-3 py-1.5 transition-colors hover:bg-sidebar-accent"
              >
                <button
                  onClick={() => navigate(`/projects/${project.id}`)}
                  className="min-w-0 flex-1 truncate text-left text-[12px] text-sidebar-foreground/70 transition-colors hover:text-sidebar-accent-foreground"
                  title={project.path}
                >
                  {project.name.length > 12 ? project.name.slice(0, 12) + '…' : project.name}
                </button>
                <button
                  onClick={() => handleLaunch(project.id, project.path)}
                  className="shrink-0 rounded p-1 opacity-0 transition-opacity group-hover:opacity-100 hover:bg-sidebar-border"
                  title="Launch Shell"
                >
                  <Terminal className="size-3.5 text-sidebar-primary" />
                </button>
              </div>
            ))
          )}
        </div>
      </div>

      <div className="mx-4 mb-4 border-t border-sidebar-border pt-3">
        <p className="text-center text-[10px] text-sidebar-foreground/30">v2.0</p>
      </div>
    </aside>
  );
}
