import * as React from "react";
import MDEditor from "@uiw/react-md-editor";
import remarkFrontmatter from "remark-frontmatter";
import { Loader2, Save, Upload, Undo2, History, PanelLeftClose, PanelLeftOpen } from "lucide-react";

import type { ResourceVersion } from "@/types/v2";
import {
  readFile,
  writeFile,
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
import { SkillFileTree } from "./SkillFileTree";
import { CodeEditor } from "./CodeEditor";
import { ImagePreview } from "./ImagePreview";
import { ReadOnlyBanner } from "./ReadOnlyBanner";

interface SkillEditorProps {
  /** Path to the skill directory */
  filePath: string;
  resourceId: string;
  /** Resource scope — "registry" means read-only */
  scope?: string;
}

const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "gif", "svg", "webp", "ico"]);
const MD_EXTS = new Set(["md", "mdx"]);
const CODE_EXTS = new Set([
  "js","jsx","ts","tsx","py","rs","go","sh","bash","zsh",
  "json","yaml","yml","toml","css","html","xml","sql","lua",
  "rb","java","c","cpp","h","hpp","swift","kt","scala",
  "txt","text","cfg","ini","conf","env",
]);

function getFileCategory(name: string): "markdown" | "code" | "image" | "unsupported" {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (MD_EXTS.has(ext)) return "markdown";
  if (IMAGE_EXTS.has(ext)) return "image";
  if (CODE_EXTS.has(ext) || ext === "") return "code";
  return "unsupported";
}

const DEFAULT_TEMPLATE = `---\nname:\ndescription:\n---\n\n`;
const previewComponents = { code: skillPreviewCode };

export function SkillEditor({ filePath, resourceId, scope }: SkillEditorProps) {
  const isReadOnly = scope === "registry";
  const skillDir = filePath;
  const skillMdPath = `${skillDir}/SKILL.md`;

  // Active file state
  const [activeFile, setActiveFile] = React.useState(skillMdPath);
  const [activeFileName, setActiveFileName] = React.useState("SKILL.md");

  // File content state
  const [content, setContent] = React.useState("");
  const [originalContent, setOriginalContent] = React.useState("");
  const [loading, setLoading] = React.useState(true);
  const [saving, setSaving] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // File tree collapsed state
  const [treeCollapsed, setTreeCollapsed] = React.useState(false);

  // Version state (only for SKILL.md)
  const [versions, setVersions] = React.useState<ResourceVersion[]>([]);
  const [currentVersion, setCurrentVersion] = React.useState<string | null>(null);
  const [isDraft, setIsDraft] = React.useState(false);
  const [publishOpen, setPublishOpen] = React.useState(false);
  const [versionsOpen, setVersionsOpen] = React.useState(false);

  const hasChanges = content !== originalContent;
  const fileCategory = getFileCategory(activeFileName);
  const isSkillMd = activeFileName === "SKILL.md";

  // Load active file
  const loadFile = React.useCallback(async (path: string, name: string) => {
    setLoading(true);
    setError(null);
    try {
      const category = getFileCategory(name);
      if (category === "image" || category === "unsupported") {
        setContent("");
        setOriginalContent("");
      } else {
        const text = await readFile(path).catch(() => (path === skillMdPath ? DEFAULT_TEMPLATE : ""));
        setContent(text);
        setOriginalContent(text);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [skillMdPath]);

  // Load versions on mount
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

  // Load initial file
  React.useEffect(() => {
    loadFile(activeFile, activeFileName);
  }, [activeFile, activeFileName, loadFile]);

  const handleSelectFile = (path: string, name: string) => {
    if (path === activeFile) return;
    if (hasChanges && !window.confirm("当前文件有未保存的更改，是否切换？")) return;
    setActiveFile(path);
    setActiveFileName(name);
  };

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      if (isSkillMd) {
        await saveSkillRawContent(resourceId, activeFile, content);
        setIsDraft(true);
      } else {
        await writeFile(activeFile, content);
      }
      setOriginalContent(content);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleDiscard = () => setContent(originalContent);

  const handlePublish = async (version: string, changelog: string) => {
    if (hasChanges) {
      await saveSkillRawContent(resourceId, skillMdPath, content);
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
    await loadFile(activeFile, activeFileName);
    const [versionList, resource] = await Promise.all([
      listResourceVersions(resourceId),
      getResource(resourceId),
    ]);
    setVersions(versionList);
    if (resource) {
      setIsDraft(resource.is_draft === 1);
      setCurrentVersion(resource.version);
    }
  };

  const handleForked = (newResourceId: string, newSourcePath: string) => {
    window.location.href = `/editor?file=${encodeURIComponent(newSourcePath)}&resource_id=${newResourceId}&type=skill&scope=library`;
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
    if (currentVersion === null)
      return <Badge variant="secondary" className="border-muted-foreground/20 bg-muted/60 text-muted-foreground">Unpublished</Badge>;
    if (isDraft)
      return <Badge variant="outline" className="border-amber-500/40 bg-amber-500/10 text-amber-600">Draft{versionSuffix}</Badge>;
    return <Badge className="border-emerald-500/30 bg-emerald-500/15 text-emerald-600">Published{versionSuffix}</Badge>;
  };

  const renderEditor = () => {
    if (loading) {
      return (
        <div className="flex flex-1 items-center justify-center">
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

    switch (fileCategory) {
      case "markdown":
        return (
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
              hideToolbar={isReadOnly}
              preview={isReadOnly ? "preview" : "live"}
            />
          </div>
        );
      case "code":
        return (
          <div className="min-h-0 flex-1">
            <CodeEditor
              value={content}
              onChange={isReadOnly ? undefined : (val) => setContent(val)}
              filename={activeFileName}
              readOnly={isReadOnly}
            />
          </div>
        );
      case "image":
        return <ImagePreview filePath={activeFile} fileName={activeFileName} />;
      default:
        return (
          <div className="flex flex-1 items-center justify-center text-muted-foreground">
            <p>此文件类型不支持预览</p>
          </div>
        );
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* Read-only banner */}
      {isReadOnly && (
        <ReadOnlyBanner
          resourceId={resourceId}
          resourceName={skillDir.split("/").pop() ?? ""}
          onForked={handleForked}
        />
      )}

      {/* Action bar — only when editable */}
      {!isReadOnly && (
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
            <Button variant="ghost" size="sm" className="h-7 gap-1 rounded-md text-xs text-muted-foreground hover:text-foreground" onClick={handleDiscard} disabled={!hasChanges}>
              <Undo2 className="size-3" /> Discard
            </Button>
            <Button size="sm" variant="outline" className="h-7 gap-1 rounded-md text-xs" onClick={handleSave} disabled={saving || !hasChanges}>
              {saving ? <Loader2 className="size-3 animate-spin" /> : <Save className="size-3" />} Save
            </Button>
            {isSkillMd && (
              <Button size="sm" className="h-7 gap-1 rounded-md text-xs" onClick={() => setPublishOpen(true)} disabled={saving}>
                <Upload className="size-3" /> Publish
              </Button>
            )}
          </div>
        </div>
      )}

      {/* Two-panel layout */}
      <div className="flex min-h-0 flex-1">
        {/* File tree sidebar */}
        {treeCollapsed ? (
          <div className="flex shrink-0 flex-col items-center border-r bg-muted/5 px-1 py-2">
            <button
              onClick={() => setTreeCollapsed(false)}
              className="rounded-md p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
              title="展开文件树"
            >
              <PanelLeftOpen className="size-4" />
            </button>
          </div>
        ) : (
          <div className="w-56 shrink-0 border-r bg-muted/5">
            <div className="flex items-center justify-end border-b px-1 py-0.5">
              <button
                onClick={() => setTreeCollapsed(true)}
                className="rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                title="折叠文件树"
              >
                <PanelLeftClose className="size-3.5" />
              </button>
            </div>
            <SkillFileTree
              rootPath={skillDir}
              selectedFile={activeFile}
              onSelectFile={handleSelectFile}
              readOnly={isReadOnly}
            />
          </div>
        )}

        {/* Editor area */}
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          {renderEditor()}
        </div>
      </div>

      {/* Status bar */}
      <div className="flex items-center gap-3 border-t bg-muted/10 px-4 py-1 text-[11px] text-muted-foreground">
        <span>{activeFile.replace(skillDir + "/", "")}</span>
        <span>{fileCategory}</span>
        {isReadOnly && <span className="text-amber-600">只读</span>}
      </div>

      {/* Publish Dialog */}
      {!isReadOnly && (
        <PublishVersionDialog
          open={publishOpen}
          onOpenChange={setPublishOpen}
          existingVersions={versions.map((v) => v.version)}
          suggestedVersion={suggestNextVersion()}
          onPublish={handlePublish}
        />
      )}
    </div>
  );
}
