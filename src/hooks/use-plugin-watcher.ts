import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { usePluginStore } from '@/stores/plugin-store';

/**
 * Watch for filesystem changes in the plugins directory and auto-rescan.
 * Debounces rapid changes (e.g., plugin update writes many files) to avoid
 * redundant scans.
 */
export function usePluginWatcher() {
  const { scanPlugins, scanning } = usePluginStore();
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const scanningRef = useRef(scanning);
  scanningRef.current = scanning;

  useEffect(() => {
    const unlisten = listen<string[]>('fs-change', (event) => {
      const hasPluginChange = event.payload.some(
        (p) =>
          p.includes('/plugins/installed_plugins.json') ||
          p.includes('/plugins/cache/'),
      );

      if (!hasPluginChange) return;

      // Debounce: wait 2s after last change before scanning
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
      }
      debounceTimer.current = setTimeout(() => {
        if (!scanningRef.current) {
          scanPlugins();
        }
      }, 2000);
    });

    return () => {
      unlisten.then((fn) => fn());
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
      }
    };
  }, [scanPlugins]);
}
