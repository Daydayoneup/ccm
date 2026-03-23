import * as React from "react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ResourceVersion } from "@/types/v2"

interface SkillVersionPanelProps {
  versions: ResourceVersion[];
  currentVersion: string | null;
  onRollback: (version: string) => Promise<void>;
}

export function SkillVersionPanel({
  versions,
  currentVersion,
  onRollback,
}: SkillVersionPanelProps) {
  const [rollingBack, setRollingBack] = React.useState<string | null>(null);

  if (versions.length === 0) {
    return (
      <p className="text-muted-foreground text-sm">No versions published yet.</p>
    );
  }

  const handleRollback = async (version: string) => {
    setRollingBack(version);
    try {
      await onRollback(version);
    } finally {
      setRollingBack(null);
    }
  };

  return (
    <div className="divide-y">
      {versions.map((v) => {
        const isCurrent = v.version === currentVersion;
        const date = new Date(v.created_at).toLocaleDateString(undefined, {
          year: "numeric",
          month: "short",
          day: "numeric",
        });

        return (
          <div key={v.id} className="flex items-start justify-between gap-4 py-3 first:pt-0 last:pb-0">
            <div className="flex flex-col gap-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="font-mono text-sm font-medium">v{v.version}</span>
                {isCurrent && (
                  <Badge variant="secondary">current</Badge>
                )}
                <span className="text-muted-foreground text-xs">{date}</span>
              </div>
              {v.changelog && (
                <p className="text-muted-foreground text-xs truncate max-w-xs">
                  {v.changelog}
                </p>
              )}
            </div>

            {!isCurrent && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleRollback(v.version)}
                disabled={rollingBack !== null}
              >
                {rollingBack === v.version ? "Rolling back..." : "Rollback"}
              </Button>
            )}
          </div>
        );
      })}
    </div>
  );
}
