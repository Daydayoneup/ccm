import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { useI18n } from '@/i18n/provider';
import { useProjectStoreV2 } from '@/stores/project-store-v2';

interface ScanDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onRegistered: () => void;
}

export function ScanDialog({ open, onOpenChange, onRegistered }: ScanDialogProps) {
  const { t, formatNumber } = useI18n();
  const { discoveredProjects, discoverProjects, registerProject } = useProjectStoreV2();
  const [scanDir, setScanDir] = useState('');
  const [scanning, setScanning] = useState(false);

  const handleScan = async () => {
    if (!scanDir.trim()) return;
    setScanning(true);
    try {
      await discoverProjects([scanDir.trim()]);
    } finally {
      setScanning(false);
    }
  };

  const handleRegister = async (path: string) => {
    await registerProject(path);
    onRegistered();
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('projects.scanDialogTitle')}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="flex gap-2">
            <Input
              value={scanDir}
              onChange={(e) => setScanDir(e.target.value)}
              placeholder={t('projects.scanDialogPlaceholder')}
            />
            <Button onClick={handleScan} disabled={scanning}>
              {scanning ? t('projects.scanning') : t('projects.scanAction')}
            </Button>
          </div>
          {discoveredProjects.length > 0 && (
            <div className="space-y-2">
              <p className="text-sm font-medium">
                {t('projects.foundProjects', { count: formatNumber(discoveredProjects.length) })}
              </p>
              {discoveredProjects.map((project) => (
                <div key={project.path} className="flex items-center justify-between rounded-md border p-3">
                  <div>
                    <div className="text-sm font-medium">{project.name}</div>
                    <div className="font-mono text-xs text-muted-foreground">{project.path}</div>
                  </div>
                  <Button size="sm" onClick={() => handleRegister(project.path)}>
                    {t('projects.register')}
                  </Button>
                </div>
              ))}
            </div>
          )}
          {discoveredProjects.length === 0 && scanDir && !scanning && (
            <p className="text-sm text-muted-foreground">{t('projects.noNewProjects')}</p>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
