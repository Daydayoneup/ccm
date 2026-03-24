import { useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { AppLayout } from '@/components/layout/AppLayout';
import { I18nProvider } from '@/i18n/provider';
import { DashboardPageV2 } from '@/pages/DashboardPage_v2';
import { GlobalPage } from '@/pages/GlobalPage';
import { ProjectListPage } from '@/pages/ProjectListPage';
import { ProjectDetailPageV2 } from '@/pages/ProjectDetailPage_v2';
import { LibraryPage_v2 } from '@/pages/LibraryPage_v2';
import { PluginListPage } from '@/pages/PluginListPage';
import { PluginDetailPage } from '@/pages/PluginDetailPage';
import { RegistryPluginDetailPage } from '@/pages/RegistryPluginDetailPage';
import { EditorPage } from '@/pages/EditorPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { useSync } from '@/hooks/use-sync';
import { usePluginWatcher } from '@/hooks/use-plugin-watcher';

function AppContent() {
  useSync();
  usePluginWatcher();

  useEffect(() => {
    // Fire-and-forget: trigger background sync on startup
    invoke('full_sync').catch((err) => {
      console.error('Background sync failed to start:', err);
    });
  }, []);

  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route path="/" element={<DashboardPageV2 />} />
        <Route path="/global" element={<GlobalPage />} />
        <Route path="/projects" element={<ProjectListPage />} />
        <Route path="/projects/:projectId" element={<ProjectDetailPageV2 />} />
        <Route path="/plugins" element={<PluginListPage />} />
        <Route path="/plugins/:id" element={<PluginDetailPage />} />
        <Route path="/registry-plugins/:id" element={<RegistryPluginDetailPage />} />
        <Route path="/library" element={<LibraryPage_v2 />} />
        <Route path="/editor" element={<EditorPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}

function App() {
  return (
    <I18nProvider>
      <BrowserRouter>
        <AppContent />
      </BrowserRouter>
    </I18nProvider>
  );
}

export default App;
