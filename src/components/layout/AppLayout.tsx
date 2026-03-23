import { useEffect, useState } from 'react';
import { Outlet } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { Sidebar } from './Sidebar';
import { CommandPalette } from '@/components/shared/CommandPalette';
import { useSyncStore } from '@/stores/sync-store';
import { RefreshCw, Check } from 'lucide-react';

function matchesShortcut(e: KeyboardEvent, shortcut: string): boolean {
  const parts = shortcut.toLowerCase().split('+');
  const key = parts[parts.length - 1];
  const needsMeta = parts.includes('meta');
  const needsCtrl = parts.includes('ctrl') || parts.includes('control');
  const needsShift = parts.includes('shift');
  const needsAlt = parts.includes('alt');
  return (
    e.key.toLowerCase() === key &&
    e.metaKey === needsMeta &&
    e.ctrlKey === needsCtrl &&
    e.shiftKey === needsShift &&
    e.altKey === needsAlt
  );
}

export function AppLayout() {
  const { syncStatus, syncProgress } = useSyncStore();
  const [showComplete, setShowComplete] = useState(false);
  const [hasRun, setHasRun] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteEnabled, setPaletteEnabled] = useState(false);
  const [shortcut, setShortcut] = useState('Meta+k');

  useEffect(() => {
    invoke<string | null>('get_app_setting', { key: 'enable_command_palette' }).then(
      (val) => setPaletteEnabled(val === 'true')
    );
    invoke<string | null>('get_app_setting', { key: 'command_palette_shortcut' }).then(
      (val) => { if (val) setShortcut(val); }
    );
  }, []);

  useEffect(() => {
    if (!paletteEnabled) return;
    const handler = (e: KeyboardEvent) => {
      if (matchesShortcut(e, shortcut)) {
        e.preventDefault();
        setPaletteOpen((prev) => !prev);
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [paletteEnabled, shortcut]);

  useEffect(() => {
    const handler = () => {
      invoke<string | null>('get_app_setting', { key: 'enable_command_palette' }).then(
        (val) => setPaletteEnabled(val === 'true')
      );
      invoke<string | null>('get_app_setting', { key: 'command_palette_shortcut' }).then(
        (val) => { if (val) setShortcut(val); }
      );
    };
    window.addEventListener('settings-changed', handler);
    return () => window.removeEventListener('settings-changed', handler);
  }, []);

  useEffect(() => {
    if (syncStatus === 'running') setHasRun(true);
  }, [syncStatus]);

  useEffect(() => {
    if (syncStatus === 'idle' && hasRun) {
      setShowComplete(true);
      const timer = setTimeout(() => setShowComplete(false), 3000);
      return () => clearTimeout(timer);
    } else {
      setShowComplete(false);
    }
  }, [syncStatus, hasRun]);

  const showBar = syncStatus === 'running' || showComplete;

  return (
    <div className="noise-bg flex h-screen overflow-hidden">
      <Sidebar paletteEnabled={paletteEnabled} onOpenPalette={() => setPaletteOpen(true)} />
      <div className="flex flex-1 flex-col overflow-hidden">
        {showBar && (
          <div className="flex items-center gap-2 border-b bg-primary/5 px-4 py-1.5 text-xs text-muted-foreground backdrop-blur-sm">
            {syncStatus === 'running' ? (
              <>
                <RefreshCw className="size-3 animate-spin text-primary" />
                <span>{syncProgress?.message || 'Syncing...'}</span>
              </>
            ) : (
              <>
                <Check className="size-3 text-chart-4" />
                <span>Sync complete</span>
              </>
            )}
          </div>
        )}
        <main className="flex flex-1 flex-col overflow-y-auto">
          <Outlet />
        </main>
      </div>
      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} />
    </div>
  );
}
