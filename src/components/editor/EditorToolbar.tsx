import * as React from "react";
import { History, Loader2, Save, Undo2, Upload } from "lucide-react";

import type { ResourceVersion } from "@/types/v2";
import {
  listResourceVersions,
  getResource,
  publishResourceVersion,
  rollbackResourceVersion,
  saveSkillRawContent,
} from "@/lib/tauri-api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { PublishVersionDialog } from "./PublishVersionDialog";

interface EditorToolbarProps {
  resourceId: string;
  skillMdPath: string;
  activeFileName: string;
  hasChanges: boolean;
  saving: boolean;
  content: string;
  isSkillMd: boolean;
  onSave: () => void;
  onDiscard: () => void;
  /** Called after publish or rollback so parent can reload file content */
  onVersionChanged: () => void;
}

export function EditorToolbar({
  resourceId,
  skillMdPath,
  activeFileName,
  hasChanges,
  saving,
  content,
  isSkillMd,
  onSave,
  onDiscard,
  onVersionChanged,
}: EditorToolbarProps) {
  const [versions, setVersions] = React.useState<ResourceVersion[]>([]);
  const [currentVersion, setCurrentVersion] = React.useState<string | null>(null);
  const [isDraft, setIsDraft] = React.useState(false);
  const [publishOpen, setPublishOpen] = React.useState(false);
  const [versionsOpen, setVersionsOpen] = React.useState(false);

  React.useEffect(() => {
    (async () => {
      try {
        const [versionList, resource] = await Promise.all([
          listResourceVersions(resourceId),
          getResource(resourceId),
        ]);
        setVersions(versionList);
        if (resource) {
          setIsDraft(resource.is_draft === 1);
          setCurrentVersion(resource.version);
        }
      } catch {
        // ignore version loading errors
      }
    })();
  }, [resourceId]);

  const suggestNextVersion = (): string => {
    if (versions.length === 0) return "1.0.0";
    const latest = versions[0].version;
    const parts = latest.split(".").map(Number);
    if (parts.length === 3 && parts.every((p) => !isNaN(p))) {
      parts[2] += 1;
      return parts.join(".");
    }
    return "1.0.0";
  };

  const handlePublish = async (version: string, changelog: string) => {
    if (hasChanges) {
      await saveSkillRawContent(resourceId, skillMdPath, content);
    }
    await publishResourceVersion(resourceId, version, changelog);
    const updated = await listResourceVersions(resourceId);
    setVersions(updated);
    setCurrentVersion(version);
    setIsDraft(false);
    onVersionChanged();
  };

  const handleRollback = async (version: string) => {
    await rollbackResourceVersion(resourceId, version);
    setVersionsOpen(false);
    const [versionList, resource] = await Promise.all([
      listResourceVersions(resourceId),
      getResource(resourceId),
    ]);
    setVersions(versionList);
    if (resource) {
      setIsDraft(resource.is_draft === 1);
      setCurrentVersion(resource.version);
    }
    onVersionChanged();
  };

  const statusBadge = () => {
    const versionSuffix = currentVersion ? ` v${currentVersion}` : "";
    if (currentVersion === null)
      return <Badge variant="secondary" className="border-muted-foreground/20 bg-muted/60 text-muted-foreground">Unpublished</Badge>;
    if (isDraft)
      return <Badge variant="outline" className="border-amber-500/40 bg-amber-500/10 text-amber-600">Draft{versionSuffix}</Badge>;
    return <Badge className="border-emerald-500/30 bg-emerald-500/15 text-emerald-600">Published{versionSuffix}</Badge>;
  };

  return (
    <>
      <div className="flex items-center gap-3 border-b bg-muted/10 px-4 py-1.5">
        {isSkillMd && statusBadge()}
        <span className="text-xs text-muted-foreground truncate">{activeFileName}</span>
        {hasChanges && (
          <div className="flex items-center gap-1.5">
            <span className="size-1.5 rounded-full bg-primary animate-pulse" />
            <span className="text-[11px] font-medium text-primary">Unsaved</span>
          </div>
        )}
        <div className="ml-auto flex items-center gap-1.5">
          {isSkillMd && (
            <Popover open={versionsOpen} onOpenChange={setVersionsOpen}>
              <PopoverTrigger asChild>
                <Button variant="outline" size="sm" className="h-7 gap-1 rounded-md text-xs">
                  <History className="size-3" />
                  Versions ({versions.length})
                </Button>
              </PopoverTrigger>
              <PopoverContent align="end" className="w-80 p-0">
                {versions.length === 0 ? (
                  <p className="p-4 text-sm text-muted-foreground">No versions published yet.</p>
                ) : (
                  <div className="max-h-60 divide-y overflow-y-auto">
                    {versions.map((v) => {
                      const isCurrent = v.version === currentVersion;
                      const date = new Date(v.created_at).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
                      return (
                        <div key={v.id} className="flex items-start justify-between gap-3 px-4 py-2.5">
                          <div className="flex min-w-0 flex-col gap-0.5">
                            <div className="flex items-center gap-2">
                              <span className="font-mono text-sm font-medium">v{v.version}</span>
                              {isCurrent && <Badge variant="secondary" className="text-[10px] h-4">current</Badge>}
                              <span className="text-xs text-muted-foreground">{date}</span>
                            </div>
                            {v.changelog && <p className="max-w-[220px] truncate text-xs text-muted-foreground">{v.changelog}</p>}
                          </div>
                          {!isCurrent && (
                            <Button variant="outline" size="sm" className="h-6 shrink-0 text-[11px]" onClick={() => handleRollback(v.version)}>
                              Rollback
                            </Button>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </PopoverContent>
            </Popover>
          )}
          <Button variant="ghost" size="sm" className="h-7 gap-1 rounded-md text-xs text-muted-foreground hover:text-foreground" onClick={onDiscard} disabled={!hasChanges}>
            <Undo2 className="size-3" /> Discard
          </Button>
          <Button size="sm" variant="outline" className="h-7 gap-1 rounded-md text-xs" onClick={onSave} disabled={saving || !hasChanges}>
            {saving ? <Loader2 className="size-3 animate-spin" /> : <Save className="size-3" />} Save
          </Button>
          {isSkillMd && (
            <Button size="sm" className="h-7 gap-1 rounded-md text-xs" onClick={() => setPublishOpen(true)} disabled={saving}>
              <Upload className="size-3" /> Publish
            </Button>
          )}
        </div>
      </div>

      <PublishVersionDialog
        open={publishOpen}
        onOpenChange={setPublishOpen}
        existingVersions={versions.map((v) => v.version)}
        suggestedVersion={suggestNextVersion()}
        onPublish={handlePublish}
      />
    </>
  );
}
