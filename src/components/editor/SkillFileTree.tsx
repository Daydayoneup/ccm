import * as React from "react";
import {
  ChevronRight,
  ChevronDown,
  File,
  Folder,
  FolderOpen,
  FilePlus,
  FolderPlus,
  Pencil,
  Trash2,
  FileCode,
  FileImage,
  FileText,
  FileJson,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  listDirectory,
  createDirectory,
  writeFile,
  deletePath,
  renamePath,
} from "@/lib/tauri-api";
import type { FileEntry } from "@/types/v2";

interface TreeNode extends FileEntry {
  children?: TreeNode[];
  loaded?: boolean;
}

function getFileIcon(name: string, isDir: boolean, isOpen: boolean) {
  if (isDir) return isOpen ? FolderOpen : Folder;
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["png", "jpg", "jpeg", "gif", "svg", "webp"].includes(ext)) return FileImage;
  if (["json", "yaml", "yml", "toml"].includes(ext)) return FileJson;
  if (["py", "js", "ts", "tsx", "jsx", "rs", "go", "sh", "bash"].includes(ext)) return FileCode;
  if (["md", "txt", "text"].includes(ext)) return FileText;
  return File;
}

interface SkillFileTreeProps {
  rootPath: string;
  selectedFile: string | null;
  onSelectFile: (path: string, name: string) => void;
  readOnly?: boolean;
  onTreeChanged?: () => void;
}

export function SkillFileTree({
  rootPath,
  selectedFile,
  onSelectFile,
  readOnly,
  onTreeChanged,
}: SkillFileTreeProps) {
  const [tree, setTree] = React.useState<TreeNode[]>([]);
  const [expanded, setExpanded] = React.useState<Set<string>>(new Set());
  const [editing, setEditing] = React.useState<{ path: string; type: "rename" | "new-file" | "new-dir" } | null>(null);
  const [editValue, setEditValue] = React.useState("");

  const loadDir = React.useCallback(async (dirPath: string): Promise<TreeNode[]> => {
    const entries = await listDirectory(dirPath);
    // Filter hidden files, sort: SKILL.md first, then dirs, then files
    return entries
      .filter((e) => !e.name.startsWith("."))
      .sort((a, b) => {
        if (a.name === "SKILL.md") return -1;
        if (b.name === "SKILL.md") return 1;
        if (a.is_dir && !b.is_dir) return -1;
        if (!a.is_dir && b.is_dir) return 1;
        return a.name.localeCompare(b.name);
      });
  }, []);

  const loadRoot = React.useCallback(async () => {
    const nodes = await loadDir(rootPath);
    setTree(nodes);
    // Auto-expand first level
    const dirs = nodes.filter((n) => n.is_dir).map((n) => n.path);
    setExpanded(new Set(dirs));
  }, [rootPath, loadDir]);

  React.useEffect(() => {
    loadRoot();
  }, [loadRoot]);

  const toggleExpand = async (node: TreeNode) => {
    const next = new Set(expanded);
    if (next.has(node.path)) {
      next.delete(node.path);
    } else {
      next.add(node.path);
      if (!node.children) {
        node.children = await loadDir(node.path);
        node.loaded = true;
        setTree([...tree]);
      }
    }
    setExpanded(next);
  };

  const handleDelete = async (path: string) => {
    await deletePath(path);
    await loadRoot();
    onTreeChanged?.();
  };

  const handleRename = async (oldPath: string) => {
    if (!editValue.trim()) { setEditing(null); return; }
    const parent = oldPath.substring(0, oldPath.lastIndexOf("/"));
    const newPath = `${parent}/${editValue.trim()}`;
    await renamePath(oldPath, newPath);
    setEditing(null);
    await loadRoot();
    onTreeChanged?.();
  };

  const handleCreate = async (parentPath: string, type: "file" | "dir") => {
    if (!editValue.trim()) { setEditing(null); return; }
    const newPath = `${parentPath}/${editValue.trim()}`;
    if (type === "dir") {
      await createDirectory(newPath);
    } else {
      await writeFile(newPath, "");
    }
    setEditing(null);
    await loadRoot();
    onTreeChanged?.();
  };

  const renderNode = (node: TreeNode, depth: number): React.ReactNode => {
    const isExpanded = expanded.has(node.path);
    const isSelected = selectedFile === node.path;
    const Icon = getFileIcon(node.name, node.is_dir, isExpanded);
    const isEditing = editing?.path === node.path && editing.type === "rename";

    const content = (
      <div
        className={cn(
          "group flex items-center gap-1 rounded-sm px-1 py-0.5 text-sm cursor-pointer hover:bg-accent",
          isSelected && "bg-accent text-accent-foreground",
        )}
        style={{ paddingLeft: `${depth * 12 + 4}px` }}
        onClick={() => {
          if (node.is_dir) toggleExpand(node);
          else onSelectFile(node.path, node.name);
        }}
      >
        {node.is_dir ? (
          isExpanded ? <ChevronDown className="size-3 shrink-0" /> : <ChevronRight className="size-3 shrink-0" />
        ) : (
          <span className="w-3" />
        )}
        <Icon className="size-3.5 shrink-0 text-muted-foreground" />
        {isEditing ? (
          <Input
            className="h-5 flex-1 px-1 py-0 text-xs"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={() => handleRename(node.path)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleRename(node.path);
              if (e.key === "Escape") setEditing(null);
            }}
            autoFocus
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <span className="truncate text-xs">{node.name}</span>
        )}
      </div>
    );

    if (readOnly) {
      return (
        <div key={node.path}>
          {content}
          {node.is_dir && isExpanded && node.children?.map((child) => renderNode(child, depth + 1))}
        </div>
      );
    }

    return (
      <div key={node.path}>
        <ContextMenu>
          <ContextMenuTrigger asChild>{content}</ContextMenuTrigger>
          <ContextMenuContent>
            {node.is_dir && (
              <>
                <ContextMenuItem onClick={() => {
                  setEditing({ path: node.path, type: "new-file" });
                  setEditValue("");
                }}>
                  <FilePlus className="mr-2 size-3.5" /> 新建文件
                </ContextMenuItem>
                <ContextMenuItem onClick={() => {
                  setEditing({ path: node.path, type: "new-dir" });
                  setEditValue("");
                }}>
                  <FolderPlus className="mr-2 size-3.5" /> 新建目录
                </ContextMenuItem>
              </>
            )}
            <ContextMenuItem onClick={() => {
              setEditing({ path: node.path, type: "rename" });
              setEditValue(node.name);
            }}>
              <Pencil className="mr-2 size-3.5" /> 重命名
            </ContextMenuItem>
            {node.name !== "SKILL.md" && (
              <ContextMenuItem className="text-destructive" onClick={() => handleDelete(node.path)}>
                <Trash2 className="mr-2 size-3.5" /> 删除
              </ContextMenuItem>
            )}
          </ContextMenuContent>
        </ContextMenu>
        {/* Inline new file/dir input */}
        {node.is_dir && isExpanded && editing?.path === node.path && editing.type !== "rename" && (
          <div className="flex items-center gap-1 px-1 py-0.5" style={{ paddingLeft: `${(depth + 1) * 12 + 4}px` }}>
            {editing.type === "new-dir" ? <Folder className="size-3.5 text-muted-foreground" /> : <File className="size-3.5 text-muted-foreground" />}
            <Input
              className="h-5 flex-1 px-1 py-0 text-xs"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={() => handleCreate(node.path, editing.type === "new-dir" ? "dir" : "file")}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate(node.path, editing.type === "new-dir" ? "dir" : "file");
                if (e.key === "Escape") setEditing(null);
              }}
              autoFocus
              placeholder={editing.type === "new-dir" ? "目录名" : "文件名"}
            />
          </div>
        )}
        {node.is_dir && isExpanded && node.children?.map((child) => renderNode(child, depth + 1))}
      </div>
    );
  };

  return (
    <div className="flex h-full flex-col overflow-y-auto">
      {!readOnly && (
        <div className="flex items-center gap-1 border-b px-2 py-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            title="新建文件"
            onClick={() => {
              setEditing({ path: rootPath, type: "new-file" });
              setEditValue("");
            }}
          >
            <FilePlus className="size-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0"
            title="新建目录"
            onClick={() => {
              setEditing({ path: rootPath, type: "new-dir" });
              setEditValue("");
            }}
          >
            <FolderPlus className="size-3.5" />
          </Button>
        </div>
      )}
      <div className="flex-1 overflow-y-auto py-1">
        {/* Root-level new file/dir input */}
        {editing?.path === rootPath && editing.type !== "rename" && (
          <div className="flex items-center gap-1 px-1 py-0.5" style={{ paddingLeft: "16px" }}>
            {editing.type === "new-dir" ? <Folder className="size-3.5 text-muted-foreground" /> : <File className="size-3.5 text-muted-foreground" />}
            <Input
              className="h-5 flex-1 px-1 py-0 text-xs"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={() => handleCreate(rootPath, editing.type === "new-dir" ? "dir" : "file")}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate(rootPath, editing.type === "new-dir" ? "dir" : "file");
                if (e.key === "Escape") setEditing(null);
              }}
              autoFocus
              placeholder={editing.type === "new-dir" ? "目录名" : "文件名"}
            />
          </div>
        )}
        {tree.map((node) => renderNode(node, 0))}
      </div>
    </div>
  );
}
