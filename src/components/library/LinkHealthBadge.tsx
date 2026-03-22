import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Heart } from 'lucide-react';

interface LinkHealthInfo {
  link_id: string;
  target_path: string;
  healthy: boolean;
  error: string | null;
}

interface LinkHealthBadgeProps {
  linkHealth: LinkHealthInfo[];
  onCheck: () => void;
  loading: boolean;
}

export function LinkHealthBadge({ linkHealth, onCheck, loading }: LinkHealthBadgeProps) {
  const brokenLinks = linkHealth.filter((l) => !l.healthy);
  const allHealthy = linkHealth.length > 0 && brokenLinks.length === 0;
  const hasBroken = brokenLinks.length > 0;

  return (
    <div className="flex items-center gap-2">
      <Button variant="outline" size="sm" onClick={onCheck} disabled={loading}>
        <Heart className="mr-1 size-4" />
        {loading ? 'Checking...' : 'Check Health'}
      </Button>

      {allHealthy && (
        <Badge variant="default" className="bg-green-600 hover:bg-green-700">
          All healthy
        </Badge>
      )}

      {hasBroken && (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="destructive">
                {brokenLinks.length} broken link{brokenLinks.length > 1 ? 's' : ''}
              </Badge>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="max-w-sm">
              <div className="space-y-1 text-xs">
                {brokenLinks.map((link) => (
                  <div key={link.link_id}>
                    <div className="font-medium truncate">{link.target_path}</div>
                    {link.error && (
                      <div className="text-destructive-foreground/70">{link.error}</div>
                    )}
                  </div>
                ))}
              </div>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
    </div>
  );
}
