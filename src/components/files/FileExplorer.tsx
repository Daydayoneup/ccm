import { useState, useEffect, useCallback } from 'react';
import { FileList } from './FileList';
import { FilePreview } from './FilePreview';
import { MarkdownEditor } from '@/components/editor/MarkdownEditor';
import { Button } from '@/components/ui/button';
import { ArrowLeft, Save, Undo2, Loader2 } from 'lucide-react';
import { readFile, writeFile } from '@/lib/tauri-api';

type ExplorerView =
  | { mode: 'list' }
  | { mode: 'preview'; filePath: string }
  | { mode: 'edit'; filePath: string };

interface FileExplorerProps {
  projectPath: string;
}

function isMarkdownOrJson(filePath: string): boolean {
  return filePath.endsWith('.md') || filePath.endsWith('.json');
}

/** Simple textarea editor for non-markdown/json files */
function PlainTextEditor({ filePath, onBack }: { filePath: string; onBack: () => void }) {
  const [content, setContent] = useState('');
  const [originalContent, setOriginalContent] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hasChanges = content !== originalContent;

  useEffect(() => {
    (async () => {
      try {
        const data = await readFile(filePath);
        setContent(data);
        setOriginalContent(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    })();
  }, [filePath]);

  const handleSave = async () => {
    setSaving(true);
    try {
      await writeFile(filePath, content);
      setOriginalContent(content);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Button variant="ghost" size="sm" onClick={onBack}>
            <ArrowLeft className="size-4 mr-1" />
            Back
          </Button>
          <span className="text-sm text-muted-foreground truncate">
            {filePath.split('/').pop()}
          </span>
          {hasChanges && (
            <span className="text-xs text-yellow-500 bg-yellow-500/10 px-2 py-0.5 rounded">
              Modified
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => setContent(originalContent)} disabled={!hasChanges}>
            <Undo2 className="size-4 mr-1" />
            Discard
          </Button>
          <Button size="sm" onClick={handleSave} disabled={!hasChanges || saving}>
            {saving ? <Loader2 className="size-4 animate-spin mr-1" /> : <Save className="size-4 mr-1" />}
            Save
          </Button>
        </div>
      </div>
      {error && (
        <div className="rounded-md border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">
          {error}
        </div>
      )}
      <textarea
        value={content}
        onChange={(e) => setContent(e.target.value)}
        className="w-full min-h-[calc(100vh-20rem)] rounded-md border bg-muted/30 p-3 font-mono text-sm resize-y focus:outline-none focus:ring-1 focus:ring-ring"
        spellCheck={false}
      />
    </div>
  );
}

export function FileExplorer({ projectPath }: FileExplorerProps) {
  const [view, setView] = useState<ExplorerView>({ mode: 'list' });

  const handleFileClick = useCallback((filePath: string) => {
    setView({ mode: 'preview', filePath });
  }, []);

  const handleBack = useCallback(() => {
    setView({ mode: 'list' });
  }, []);

  const handleEdit = useCallback((filePath: string) => {
    setView({ mode: 'edit', filePath });
  }, []);

  const handleFileCreate = useCallback((filePath: string) => {
    setView({ mode: 'edit', filePath });
  }, []);

  switch (view.mode) {
    case 'list':
      return (
        <FileList
          rootPath={projectPath}
          onFileClick={handleFileClick}
          onFileCreate={handleFileCreate}
        />
      );

    case 'preview':
      return (
        <FilePreview
          filePath={view.filePath}
          rootPath={projectPath}
          onBack={handleBack}
          onEdit={() => handleEdit(view.filePath)}
        />
      );

    case 'edit':
      if (isMarkdownOrJson(view.filePath)) {
        return (
          <div className="space-y-3">
            <Button variant="ghost" size="sm" onClick={handleBack}>
              <ArrowLeft className="size-4 mr-1" />
              Back
            </Button>
            <MarkdownEditor filePath={view.filePath} />
          </div>
        );
      }
      return <PlainTextEditor filePath={view.filePath} onBack={handleBack} />;
  }
}
