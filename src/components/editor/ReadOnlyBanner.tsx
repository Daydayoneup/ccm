import * as React from "react";
import { Lock, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ForkToLibraryDialog } from "./ForkToLibraryDialog";

interface ReadOnlyBannerProps {
  registryName?: string;
  resourceId: string;
  resourceName: string;
  onForked: (newResourceId: string, newSourcePath: string) => void;
}

export function ReadOnlyBanner({
  registryName,
  resourceId,
  resourceName,
  onForked,
}: ReadOnlyBannerProps) {
  const [dialogOpen, setDialogOpen] = React.useState(false);

  return (
    <>
      <div className="flex items-center gap-2 border-b bg-amber-50 px-4 py-2 text-sm text-amber-800">
        <Lock className="size-3.5 shrink-0" />
        <span>
          此资源来自仓库{registryName ? ` ${registryName}` : ""}，不可编辑。
        </span>
        <Button
          variant="outline"
          size="sm"
          className="ml-auto h-6 gap-1 text-xs"
          onClick={() => setDialogOpen(true)}
        >
          <Copy className="size-3" />
          复制到本地库
        </Button>
      </div>
      <ForkToLibraryDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        resourceId={resourceId}
        defaultName={resourceName}
        onForked={onForked}
      />
    </>
  );
}
