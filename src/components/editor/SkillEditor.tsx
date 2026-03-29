import * as React from "react";
import MDEditor from "@uiw/react-md-editor";
import remarkFrontmatter from "remark-frontmatter";
import { Loader2, PanelLeftClose, PanelLeftOpen } from "lucide-react";

import {
  readFile,
  writeFile,
  saveSkillRawContent,
  pathIsDirectory,
} from "@/lib/tauri-api";
import { cn } from "@/lib/utils";
import { EditorToolbar } from "./EditorToolbar";
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

  // Whether filePath is a single file (e.g., existing-skill.md) or directory (e.g., my-skill/)
  const [isSingleFile, setIsSingleFile] = React.useState<boolean | null>(null);

  // Derived paths — computed once isSingleFile is known
  const skillDir = isSingleFile ? filePath.substring(0, filePath.lastIndexOf("/")) : filePath;
  const skillMdPath = isSingleFile ? filePath : `${filePath}/SKILL.md`;
  // Active file state
  const [activeFile, setActiveFile] = React.useState(filePath);
  const [activeFileName, setActiveFileName] = React.useState("SKILL.md");

  // File content state
  const [content, setContent] = React.useState("");
  const [originalContent, setOriginalContent] = React.useState("");
  const [loading, setLoading] = React.useState(true);
  const [saving, setSaving] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // File tree collapsed state
  const [treeCollapsed, setTreeCollapsed] = React.useState(false);

  // Detect file vs directory on mount
  React.useEffect(() => {
    pathIsDirectory(filePath).then((isDir) => {
      const single = !isDir;
      setIsSingleFile(single);
      const mdPath = single ? filePath : `${filePath}/SKILL.md`;
      const fileName = single ? (filePath.split("/").pop() ?? "SKILL.md") : "SKILL.md";
      setActiveFile(mdPath);
      setActiveFileName(fileName);
    });
  }, [filePath]);

  const hasChanges = content !== originalContent;
  const fileCategory = getFileCategory(activeFileName);
  const isSkillMd = isSingleFile === true || activeFileName === "SKILL.md";

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

  // Load initial file (wait for isSingleFile detection)
  React.useEffect(() => {
    if (isSingleFile === null) return;
    loadFile(activeFile, activeFileName);
  }, [activeFile, activeFileName, loadFile, isSingleFile]);

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

  const handleForked = (newResourceId: string, newSourcePath: string) => {
    window.location.href = `/editor?file=${encodeURIComponent(newSourcePath)}&resource_id=${newResourceId}&type=skill&scope=library`;
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
        <EditorToolbar
          resourceId={resourceId}
          skillMdPath={skillMdPath}
          activeFileName={activeFileName}
          hasChanges={hasChanges}
          saving={saving}
          content={content}
          isSkillMd={isSkillMd}
          onSave={handleSave}
          onDiscard={handleDiscard}
          onVersionChanged={() => loadFile(activeFile, activeFileName)}
        />
      )}

      {/* Two-panel layout */}
      <div className="flex min-h-0 flex-1">
        {/* File tree sidebar — hidden for single-file skills */}
        {isSingleFile ? null : treeCollapsed ? (
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

    </div>
  );
}
