import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { ArrowLeft, Pencil, Loader2 } from 'lucide-react';
import { readFile } from '@/lib/tauri-api';

interface FilePreviewProps {
  filePath: string;
  rootPath: string;
  onBack: () => void;
  onEdit: () => void;
}

const MAX_FILE_SIZE = 1024 * 1024; // 1MB

function isBinaryContent(content: string): boolean {
  // Check first 8KB for null bytes or high ratio of non-printable chars
  const sample = content.slice(0, 8192);
  let nonPrintable = 0;
  for (let i = 0; i < sample.length; i++) {
    const code = sample.charCodeAt(i);
    if (code === 0) return true;
    if (code < 32 && code !== 9 && code !== 10 && code !== 13) nonPrintable++;
  }
  return sample.length > 0 && nonPrintable / sample.length > 0.1;
}

export function FilePreview({ filePath, rootPath, onBack, onEdit }: FilePreviewProps) {
  const [content, setContent] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isBinary, setIsBinary] = useState(false);

  const relativePath = filePath.startsWith(rootPath)
    ? filePath.slice(rootPath.length).replace(/^\//, '')
    : filePath;

  const loadFile = useCallback(async () => {
    setLoading(true);
    setError(null);
    setIsBinary(false);
    try {
      const data = await readFile(filePath);
      if (data.length > MAX_FILE_SIZE) {
        setError('File is too large to preview (>1MB).');
        return;
      }
      if (isBinaryContent(data)) {
        setIsBinary(true);
        return;
      }
      setContent(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [filePath]);

  useEffect(() => {
    loadFile();
  }, [loadFile]);

  const lines = content.split('\n');

  return (
    <div className="space-y-3">
      {/* Header bar */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Button variant="ghost" size="sm" onClick={onBack}>
            <ArrowLeft className="size-4 mr-1" />
            Back
          </Button>
          <span className="text-sm text-muted-foreground truncate">{relativePath}</span>
        </div>
        {!loading && !error && !isBinary && (
          <Button size="sm" onClick={onEdit}>
            <Pencil className="size-4 mr-1" />
            Edit
          </Button>
        )}
      </div>

      {/* Content */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="size-6 animate-spin text-muted-foreground" />
        </div>
      ) : error ? (
        <div className="rounded-md border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : isBinary ? (
        <div className="text-center py-12 text-muted-foreground text-sm">
          This file appears to be binary and cannot be previewed.
        </div>
      ) : (
        <div className="rounded-md border overflow-auto max-h-[calc(100vh-16rem)]">
          <pre className="text-sm leading-relaxed p-0 m-0">
            <table className="w-full border-collapse">
              <tbody>
                {lines.map((line, i) => (
                  <tr key={i} className="hover:bg-muted/30">
                    <td className="px-3 py-0 text-right text-muted-foreground select-none border-r w-12 align-top text-xs">
                      {i + 1}
                    </td>
                    <td className="px-3 py-0 whitespace-pre font-mono text-xs">
                      {line || '\u00A0'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </pre>
        </div>
      )}
    </div>
  );
}
