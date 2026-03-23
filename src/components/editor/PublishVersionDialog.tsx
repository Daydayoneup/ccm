import * as React from "react"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"

interface PublishVersionDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  existingVersions: string[];
  suggestedVersion: string;
  onPublish: (version: string, changelog: string) => Promise<void>;
}

const SEMVER_REGEX = /^\d+\.\d+\.\d+$/;

export function PublishVersionDialog({
  open,
  onOpenChange,
  existingVersions,
  suggestedVersion,
  onPublish,
}: PublishVersionDialogProps) {
  const [version, setVersion] = React.useState(suggestedVersion);
  const [changelog, setChangelog] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);
  const [publishing, setPublishing] = React.useState(false);

  // Sync suggestedVersion when dialog opens
  React.useEffect(() => {
    if (open) {
      setVersion(suggestedVersion);
      setChangelog("");
      setError(null);
    }
  }, [open, suggestedVersion]);

  const validate = (v: string): string | null => {
    if (!SEMVER_REGEX.test(v)) {
      return "Version must be in semver format (e.g. 1.0.0)";
    }
    if (existingVersions.includes(v)) {
      return `Version ${v} already exists`;
    }
    return null;
  };

  const handleVersionChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = e.target.value;
    setVersion(v);
    setError(validate(v));
  };

  const handlePublish = async () => {
    const validationError = validate(version);
    if (validationError) {
      setError(validationError);
      return;
    }
    setPublishing(true);
    try {
      await onPublish(version, changelog);
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to publish version");
    } finally {
      setPublishing(false);
    }
  };

  const isInvalid = !SEMVER_REGEX.test(version) || existingVersions.includes(version);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Publish New Version</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="version-input">Version</Label>
            <Input
              id="version-input"
              value={version}
              onChange={handleVersionChange}
              placeholder="1.0.0"
              aria-invalid={error !== null}
            />
            {error && (
              <p className="text-destructive text-sm">{error}</p>
            )}
          </div>

          <div className="flex flex-col gap-1.5">
            <Label htmlFor="changelog-input">Changelog <span className="text-muted-foreground">(optional)</span></Label>
            <Textarea
              id="changelog-input"
              value={changelog}
              onChange={(e) => setChangelog(e.target.value)}
              placeholder="Describe what changed in this version..."
              rows={4}
            />
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={publishing}
          >
            Cancel
          </Button>
          <Button
            onClick={handlePublish}
            disabled={isInvalid || publishing}
          >
            {publishing ? "Publishing..." : "Publish"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
