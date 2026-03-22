import { useState, useEffect, useCallback } from 'react';
import MDEditor from '@uiw/react-md-editor';
import remarkFrontmatter from 'remark-frontmatter';
import type { Root } from 'mdast';
import { Save, Undo2, Loader2 } from 'lucide-react';

/** Converts YAML frontmatter AST nodes into fenced code blocks so they remain visible in preview. */
function remarkFrontmatterToCode() {
  return (tree: Root) => {
    for (let i = 0; i < tree.children.length; i++) {
      const node = tree.children[i];
      if (node.type === 'yaml') {
        tree.children[i] = {
          type: 'code',
          lang: 'yaml',
          value: node.value,
        };
      }
    }
  };
}

/** For JSON files, replace the entire preview AST with a pretty-printed code block. */
function remarkJsonPreview(rawContent: string) {
  return () => (tree: Root) => {
    let formatted: string;
    try {
      formatted = JSON.stringify(JSON.parse(rawContent), null, 2);
    } catch {
      formatted = rawContent;
    }
    tree.children = [{ type: 'code', lang: 'json', value: formatted }];
  };
}
import { Button } from '@/components/ui/button';
import { readFile, writeFile } from '@/lib/tauri-api';

interface MarkdownEditorProps {
  filePath: string;
}

export function MarkdownEditor({ filePath }: MarkdownEditorProps) {
  const [content, setContent] = useState<string>('');
  const [originalContent, setOriginalContent] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hasChanges = content !== originalContent;
  const isJson = filePath.endsWith('.json');

  const loadFile = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await readFile(filePath);
      setContent(data);
      setOriginalContent(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [filePath]);

  useEffect(() => {
    loadFile();
  }, [loadFile]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await writeFile(filePath, content);
      setOriginalContent(content);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleDiscard = () => {
    setContent(originalContent);
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
      {/* Action bar */}
      <div className="flex items-center justify-end gap-2 border-b bg-muted/20 px-4 py-1.5">
        {hasChanges && (
          <span className="mr-auto text-[11px] font-medium text-primary">Unsaved changes</span>
        )}
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1 rounded-lg text-xs text-muted-foreground hover:text-foreground"
          onClick={handleDiscard}
          disabled={!hasChanges}
        >
          <Undo2 className="size-3" />
          Discard
        </Button>
        <Button
          size="sm"
          className="h-7 gap-1 rounded-lg text-xs"
          onClick={handleSave}
          disabled={!hasChanges || saving}
        >
          {saving ? (
            <Loader2 className="size-3 animate-spin" />
          ) : (
            <Save className="size-3" />
          )}
          Save
        </Button>
      </div>

      {/* Editor */}
      <div data-color-mode="light" className="min-h-0 flex-1 [&_.w-md-editor]:!h-full">
        <MDEditor
          value={content}
          onChange={(val) => setContent(val ?? '')}
          height="100%"
          previewOptions={{
            remarkPlugins: isJson
              ? [remarkJsonPreview(content)]
              : [remarkFrontmatter, remarkFrontmatterToCode],
          }}
        />
      </div>
    </div>
  );
}
