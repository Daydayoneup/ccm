import * as React from "react";
import MDEditor from "@uiw/react-md-editor";
import remarkFrontmatter from "remark-frontmatter";
import { Loader2, Save, Upload, Undo2, History } from "lucide-react";

import type { ResourceVersion } from "@/types/v2";
import {
  readFile,
  saveSkillRawContent,
  publishResourceVersion,
  listResourceVersions,
  rollbackResourceVersion,
  getResource,
} from "@/lib/tauri-api";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { PublishVersionDialog } from "./PublishVersionDialog";
import {
  remarkFrontmatterCard,
  skillPreviewCode,
} from "./remarkFrontmatterCard";

interface SkillEditorProps {
  filePath: string;
  resourceId: string;
}

const DEFAULT_TEMPLATE = `---
name:
description:
---

`;

const previewComponents = { code: skillPreviewCode };

export function SkillEditor({ filePath, resourceId }: SkillEditorProps) {
  const [content, setContent] = React.useState("");
  const [originalContent, setOriginalContent] = React.useState("");
  const [versions, setVersions] = React.useState<ResourceVersion[]>([]);
  const [currentVersion, setCurrentVersion] = React.useState<string | null>(
    null,
  );
  const [isDraft, setIsDraft] = React.useState(false);
  const [loading, setLoading] = React.useState(true);
  const [saving, setSaving] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [publishOpen, setPublishOpen] = React.useState(false);
  const [versionsOpen, setVersionsOpen] = React.useState(false);

  const hasChanges = content !== originalContent;

  const load = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [fileContent, versionList] = await Promise.all([
        readFile(filePath).catch(() => ""),
        listResourceVersions(resourceId),
      ]);
      const text = fileContent || DEFAULT_TEMPLATE;
      setContent(text);
      setOriginalContent(text);
      setVersions(versionList);

      const resource = await getResource(resourceId);
      if (resource) {
        setIsDraft(resource.is_draft === 1);
        setCurrentVersion(resource.version);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [filePath, resourceId]);

  React.useEffect(() => {
    load();
  }, [load]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await saveSkillRawContent(resourceId, filePath, content);
      setOriginalContent(content);
      setIsDraft(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleDiscard = () => {
    setContent(originalContent);
  };

  const handlePublish = async (version: string, changelog: string) => {
    if (hasChanges) {
      await saveSkillRawContent(resourceId, filePath, content);
      setOriginalContent(content);
    }
    await publishResourceVersion(resourceId, version, changelog);
    const updated = await listResourceVersions(resourceId);
    setVersions(updated);
    setCurrentVersion(version);
    setIsDraft(false);
  };

  const handleRollback = async (version: string) => {
    await rollbackResourceVersion(resourceId, version);
    setVersionsOpen(false);
    await load();
  };

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

  const statusBadge = () => {
    const versionSuffix = currentVersion ? ` v${currentVersion}` : "";
    if (currentVersion === null) {
      return (
        <Badge
          variant="secondary"
          className="border-muted-foreground/20 bg-muted/60 text-muted-foreground"
        >
          Unpublished
        </Badge>
      );
    }
    if (isDraft) {
      return (
        <Badge
          variant="outline"
          className="border-amber-500/40 bg-amber-500/10 text-amber-600"
        >
          Draft{versionSuffix}
        </Badge>
      );
    }
    return (
      <Badge className="border-emerald-500/30 bg-emerald-500/15 text-emerald-600">
        Published{versionSuffix}
      </Badge>
    );
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2 className="size-6 animate-spin text-primary" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="m-4 rounded-xl border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
        {error}
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* Action Bar */}
      <div className="flex items-center gap-3 border-b bg-muted/10 px-4 py-1.5">
        {statusBadge()}
        {hasChanges && (
          <div className="flex items-center gap-1.5">
            <span className="size-1.5 rounded-full bg-primary animate-pulse" />
            <span className="text-[11px] font-medium text-primary">
              Unsaved
            </span>
          </div>
        )}
        <div className="ml-auto flex items-center gap-1.5">
          {/* Versions Popover */}
          <Popover open={versionsOpen} onOpenChange={setVersionsOpen}>
            <PopoverTrigger asChild>
              <Button
                variant="outline"
                size="sm"
                className="h-7 gap-1 rounded-md text-xs"
              >
                <History className="size-3" />
                Versions ({versions.length})
              </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="w-80 p-0">
              {versions.length === 0 ? (
                <p className="p-4 text-sm text-muted-foreground">
                  No versions published yet.
                </p>
              ) : (
                <div className="max-h-60 divide-y overflow-y-auto">
                  {versions.map((v) => {
                    const isCurrent = v.version === currentVersion;
                    const date = new Date(v.created_at).toLocaleDateString(
                      undefined,
                      { year: "numeric", month: "short", day: "numeric" },
                    );
                    return (
                      <div
                        key={v.id}
                        className="flex items-start justify-between gap-3 px-4 py-2.5"
                      >
                        <div className="flex min-w-0 flex-col gap-0.5">
                          <div className="flex items-center gap-2">
                            <span className="font-mono text-sm font-medium">
                              v{v.version}
                            </span>
                            {isCurrent && (
                              <Badge variant="secondary" className="text-[10px] h-4">
                                current
                              </Badge>
                            )}
                            <span className="text-xs text-muted-foreground">
                              {date}
                            </span>
                          </div>
                          {v.changelog && (
                            <p className="max-w-[220px] truncate text-xs text-muted-foreground">
                              {v.changelog}
                            </p>
                          )}
                        </div>
                        {!isCurrent && (
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-6 shrink-0 text-[11px]"
                            onClick={() => handleRollback(v.version)}
                          >
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

          {/* Discard */}
          <Button
            variant="ghost"
            size="sm"
            className="h-7 gap-1 rounded-md text-xs text-muted-foreground hover:text-foreground"
            onClick={handleDiscard}
            disabled={!hasChanges}
          >
            <Undo2 className="size-3" />
            Discard
          </Button>

          {/* Save */}
          <Button
            size="sm"
            variant="outline"
            className="h-7 gap-1 rounded-md text-xs"
            onClick={handleSave}
            disabled={saving || !hasChanges}
          >
            {saving ? (
              <Loader2 className="size-3 animate-spin" />
            ) : (
              <Save className="size-3" />
            )}
            Save
          </Button>

          {/* Publish */}
          <Button
            size="sm"
            className="h-7 gap-1 rounded-md text-xs"
            onClick={() => setPublishOpen(true)}
            disabled={saving}
          >
            <Upload className="size-3" />
            Publish
          </Button>
        </div>
      </div>

      {/* Full-screen MDEditor */}
      <div
        data-color-mode="light"
        className={cn(
          "min-h-0 h-0 flex-1",
          "[&_.w-md-editor]:!h-full [&_.w-md-editor]:!rounded-none [&_.w-md-editor]:!border-0",
          "[&_.w-md-editor>.w-md-editor-content]:!h-[calc(100%-29px)]",
        )}
      >
        <MDEditor
          value={content}
          onChange={(val) => setContent(val ?? "")}
          previewOptions={{
            remarkPlugins: [remarkFrontmatter, remarkFrontmatterCard],
            components: previewComponents,
          }}
        />
      </div>

      {/* Publish Dialog */}
      <PublishVersionDialog
        open={publishOpen}
        onOpenChange={setPublishOpen}
        existingVersions={versions.map((v) => v.version)}
        suggestedVersion={suggestNextVersion()}
        onPublish={handlePublish}
      />
    </div>
  );
}
