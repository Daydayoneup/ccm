import { useEffect, useState } from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { getAppSetting } from '@/lib/tauri-api';
import { RefreshCw, Check } from 'lucide-react';
import { Sidebar } from './Sidebar';
import { CommandPalette } from '@/components/shared/CommandPalette';
import { useSyncStore } from '@/stores/sync-store';
import { useI18n } from '@/i18n/provider';

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
  const { t } = useI18n();
  const location = useLocation();
  const { syncStatus, syncProgress } = useSyncStore();
  const [showComplete, setShowComplete] = useState(false);
  const [hasRun, setHasRun] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteEnabled, setPaletteEnabled] = useState(false);
  const [shortcut, setShortcut] = useState('Meta+k');

  useEffect(() => {
    getAppSetting('enable_command_palette').then(
      (val) => setPaletteEnabled(val === 'true')
    );
    getAppSetting('command_palette_shortcut').then(
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
      getAppSetting('enable_command_palette').then(
        (val) => setPaletteEnabled(val === 'true')
      );
      getAppSetting('command_palette_shortcut').then(
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
    }
    setShowComplete(false);
  }, [syncStatus, hasRun]);

  const routeLabel =
    location.pathname === '/'
      ? t('nav.dashboard')
      : location.pathname.startsWith('/projects')
        ? t('nav.projects')
        : location.pathname.startsWith('/global')
          ? t('nav.global')
          : location.pathname.startsWith('/library')
            ? t('nav.library')
            : location.pathname.startsWith('/settings')
              ? t('nav.settings')
              : t('common.appName');

  const showBar = syncStatus === 'running' || showComplete;

  return (
    <div className="noise-bg flex h-screen overflow-hidden">
      <Sidebar paletteEnabled={paletteEnabled} onOpenPalette={() => setPaletteOpen(true)} />
      <div className="flex flex-1 flex-col overflow-hidden">
        <header className="flex items-center justify-between border-b border-border/60 px-6 py-4">
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-muted-foreground/70">
              {t('common.appName')}
            </p>
            <h2 className="mt-1 text-sm font-medium text-foreground">{routeLabel}</h2>
          </div>
          {showBar ? (
            <div className="flex items-center gap-2 rounded-full border border-border/60 bg-panel px-3 py-1.5 text-xs text-muted-foreground shadow-sm backdrop-blur-sm">
              {syncStatus === 'running' ? (
                <>
                  <RefreshCw className="size-3 animate-spin text-primary" />
                  <span>{syncProgress?.message || t('common.syncing')}</span>
                </>
              ) : (
                <>
                  <Check className="size-3 text-chart-4" />
                  <span>{t('common.syncComplete')}</span>
                </>
              )}
            </div>
          ) : null}
        </header>
        <main className="flex flex-1 flex-col overflow-y-auto">
          <Outlet />
        </main>
      </div>
      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} />
    </div>
  );
}
