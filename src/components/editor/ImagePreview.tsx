import { convertFileSrc } from "@tauri-apps/api/core";

interface ImagePreviewProps {
  filePath: string;
  fileName: string;
}

export function ImagePreview({ filePath, fileName }: ImagePreviewProps) {
  const src = convertFileSrc(filePath);

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-4 p-8">
      <img
        src={src}
        alt={fileName}
        className="max-h-[70vh] max-w-full rounded-lg border object-contain shadow-sm"
      />
      <p className="text-sm text-muted-foreground">{fileName}</p>
    </div>
  );
}
