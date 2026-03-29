import { type ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import type { Resource, ResourceLink } from '@/types/v2';
import { ResourceInstallDropdown } from '@/components/registry/ResourceInstallDropdown';
import { ResourceUninstallPopover } from '@/components/registry/ResourceUninstallPopover';

export type ResourceInstallStatus =
  | 'not_installed'
  | 'up_to_date'
  | 'update_available'
  | 'update_conflict'
  | 'removed'
  | 'removed_local_modified';

export function getResourceInstallStatus(
  resource: Resource,
  links: ResourceLink[],
): ResourceInstallStatus {
  if (links.length === 0) return 'not_installed';
  const installedHash = links[0]?.installed_hash;
  const isRemoved = resource.is_draft === -1;
  if (isRemoved) return 'removed';
  const hasUpdate =
    resource.content_hash != null &&
    installedHash != null &&
    resource.content_hash !== installedHash;
  if (hasUpdate) return 'update_available';
  return 'up_to_date';
}

const statusBadgeConfig: Record<string, { label: string; className: string } | null> = {
  not_installed: null,
  up_to_date: { label: '已安装', className: '' },
  update_available: { label: '有更新', className: 'bg-blue-500 text-white hover:bg-blue-600' },
  update_conflict: { label: '有更新（本地已修改）', className: 'bg-orange-500 text-white hover:bg-orange-600' },
  removed: { label: '上游已移除', className: 'bg-red-500 text-white hover:bg-red-600' },
  removed_local_modified: { label: '上游已移除（本地已修改）', className: 'bg-red-500 text-white hover:bg-red-600' },
};

interface ResourceCardProps {
  resource: Resource;
  /** Install status; defaults to 'not_installed' */
  status?: ResourceInstallStatus;
  /** Links for this resource (needed for uninstall popover) */
  links?: ResourceLink[];
  /** Project id → name mapping for uninstall popover */
  projectNames?: Record<string, string>;
  /** Called when user picks "安装到项目" */
  onInstallToProject?: () => void;
  /** Called when user picks "安装到全局" */
  onInstallToGlobal?: () => void;
  /** Called when user confirms uninstall */
  onUninstall?: (linkIds: string[]) => void;
  /** Called when user clicks "更新" */
  onUpdate?: () => void;
  /** Called when user clicks "保留" */
  onRetain?: () => void;
  /** Extra action buttons rendered after install/uninstall actions */
  extraActions?: ReactNode;
  /** Called when the card body (name/description area) is clicked */
  onClick?: () => void;
}

export function ResourceCard({
  resource,
  status = 'not_installed',
  links = [],
  projectNames = {},
  onInstallToProject,
  onInstallToGlobal,
  onUninstall,
  onUpdate,
  onRetain,
  extraActions,
  onClick,
}: ResourceCardProps) {
  const badge = statusBadgeConfig[status];

  return (
    <div className={`rounded-md border bg-card/90 p-4${onClick ? ' cursor-pointer hover:bg-muted/30 transition-colors' : ''}`} onClick={onClick}>
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="truncate text-sm font-medium">{resource.name}</span>
            {badge && (
              <Badge variant="secondary" className={`shrink-0 text-xs ${badge.className}`}>
                {badge.label}
              </Badge>
            )}
          </div>
          <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">
            {resource.description || '暂无描述'}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {/* Install dropdown: always show when callbacks provided */}
          {onInstallToProject && onInstallToGlobal && (
            <ResourceInstallDropdown
              onInstallToProject={onInstallToProject}
              onInstallToGlobal={onInstallToGlobal}
            />
          )}
          {/* Update button: only when update available */}
          {(status === 'update_available' || status === 'update_conflict') && onUpdate && (
            <button
              className="rounded-md px-2 py-1 text-xs font-medium text-blue-500 hover:bg-blue-500/10"
              onClick={(e) => { e.stopPropagation(); onUpdate(); }}
            >
              更新
            </button>
          )}
          {/* Retain button: only when upstream removed */}
          {(status === 'removed' || status === 'removed_local_modified') && onRetain && (
            <button
              className="rounded-md px-2 py-1 text-xs font-medium text-primary hover:bg-primary/10"
              onClick={(e) => { e.stopPropagation(); onRetain(); }}
            >
              保留
            </button>
          )}
          {/* Uninstall popover: when installed with links */}
          {onUninstall && links.length > 0 && (
            <ResourceUninstallPopover
              links={links}
              projectNames={projectNames}
              onUninstall={onUninstall}
            />
          )}
          {extraActions}
        </div>
      </div>
    </div>
  );
}
