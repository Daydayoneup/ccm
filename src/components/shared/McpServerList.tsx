import { Badge } from '@/components/ui/badge';
import { ScopeBadge } from '@/lib/scope-utils';
import { Server } from 'lucide-react';

export interface McpServerInfo {
  id: string;
  name: string;
  server_type: string | null;
  command: string | null;
  args: string | null;
  url: string | null;
  source_path: string;
}

interface McpServerListProps {
  servers: McpServerInfo[];
  emptyMessage?: string;
  emptyHint?: string;
  onServerClick?: (server: McpServerInfo) => void;
  globalServerIds?: Set<string>;
}

export function McpServerList({
  servers,
  emptyMessage = 'No MCP servers configured.',
  emptyHint,
  onServerClick,
  globalServerIds,
}: McpServerListProps) {
  if (servers.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Server className="mb-4 size-12" />
        <p>{emptyMessage}</p>
        {emptyHint && <p className="text-sm">{emptyHint}</p>}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {servers.map((server) => {
        const isGlobal = globalServerIds?.has(server.id) ?? false;
        return (
        <div
          key={server.id}
          className={`rounded-lg border p-4${isGlobal ? ' cursor-pointer hover:bg-muted/50' : ''}`}
          onClick={isGlobal ? () => onServerClick?.(server) : undefined}
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Server className="size-4 text-muted-foreground" />
              <span className="font-medium">{server.name}</span>
              {isGlobal && <ScopeBadge scope="global" />}
            </div>
            {server.server_type && (
              <Badge variant="secondary">{server.server_type}</Badge>
            )}
          </div>
          <div className="mt-2 space-y-1 text-sm text-muted-foreground">
            {server.command && (
              <p>
                <span className="font-medium">Command:</span> {server.command}{' '}
                {server.args ? JSON.parse(server.args).join(' ') : ''}
              </p>
            )}
            {server.url && (
              <p>
                <span className="font-medium">URL:</span> {server.url}
              </p>
            )}
            <p className="truncate">
              <span className="font-medium">Source:</span> {server.source_path}
            </p>
          </div>
        </div>
        );
      })}
    </div>
  );
}
