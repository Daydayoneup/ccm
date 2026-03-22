import { useSearchParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, FileText } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { MarkdownEditor } from '@/components/editor/MarkdownEditor';

function extractFileName(filePath: string): { name: string; dir: string } {
  const parts = filePath.split('/');
  const name = parts.pop() ?? filePath;
  const dir = parts.join('/');
  return { name, dir };
}

export function EditorPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const filePath = searchParams.get('file');

  if (!filePath) {
    return (
      <div className="flex items-center justify-center p-16">
        <p className="text-muted-foreground">
          No file specified. Use the <code className="font-mono text-xs">?file=</code> query parameter.
        </p>
      </div>
    );
  }

  const { name, dir } = extractFileName(filePath);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* Top bar */}
      <div className="flex items-center gap-3 border-b px-4 py-2.5">
        <Button
          variant="ghost"
          size="icon-sm"
          className="shrink-0 rounded-lg text-muted-foreground hover:text-foreground"
          onClick={() => navigate(-1)}
        >
          <ArrowLeft className="size-4" />
        </Button>
        <div className="h-5 w-px bg-border" />
        <div className="flex items-center gap-2 min-w-0">
          <FileText className="size-4 shrink-0 text-primary" />
          <span className="truncate text-sm font-semibold">{name}</span>
          <span className="truncate font-mono text-[11px] text-muted-foreground/50">{dir}/</span>
        </div>
      </div>

      {/* Editor fills remaining space */}
      <div className="min-h-0 flex-1">
        <MarkdownEditor filePath={filePath} />
      </div>
    </div>
  );
}
