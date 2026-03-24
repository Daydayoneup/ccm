import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, vi } from 'vitest';
import { Sidebar } from '../Sidebar';

vi.mock('@/stores/project-store-v2', () => ({
  useProjectStoreV2: () => ({
    projects: [],
    loadProjects: vi.fn(),
  }),
}));

describe('Sidebar', () => {
  function renderSidebar(initialRoute = '/') {
    Object.defineProperty(window.navigator, 'language', {
      configurable: true,
      value: 'zh-CN',
    });
    return render(
      <MemoryRouter initialEntries={[initialRoute]}>
        <Sidebar paletteEnabled={false} onOpenPalette={() => {}} />
      </MemoryRouter>
    );
  }

  it('renders all navigation links', () => {
    renderSidebar();

    expect(screen.getByText('仪表盘')).toBeInTheDocument();
    expect(screen.getByText('全局资源')).toBeInTheDocument();
    expect(screen.getByText('项目')).toBeInTheDocument();
    expect(screen.getByText('资源库')).toBeInTheDocument();
    expect(screen.getByText('设置')).toBeInTheDocument();
  });

  it('renders the app title', () => {
    renderSidebar();

    expect(screen.getByText('CCM')).toBeInTheDocument();
  });

  it('links point to correct routes', () => {
    renderSidebar();

    const dashboardLink = screen.getByText('仪表盘').closest('a');
    const globalLink = screen.getByText('全局资源').closest('a');
    const projectsLink = screen.getByText('项目').closest('a');
    const libraryLink = screen.getByText('资源库').closest('a');
    const settingsLink = screen.getByText('设置').closest('a');

    expect(dashboardLink).toHaveAttribute('href', '/');
    expect(globalLink).toHaveAttribute('href', '/global');
    expect(projectsLink).toHaveAttribute('href', '/projects');
    expect(libraryLink).toHaveAttribute('href', '/library');
    expect(settingsLink).toHaveAttribute('href', '/settings');
  });

  it('highlights active link for Dashboard', () => {
    renderSidebar('/');

    const dashboardLink = screen.getByText('仪表盘').closest('a');
    expect(dashboardLink?.className).toContain('bg-sidebar-primary');
  });

  it('highlights active link for Library', () => {
    renderSidebar('/library');

    const libraryLink = screen.getByText('资源库').closest('a');
    expect(libraryLink?.className).toContain('bg-sidebar-primary');

    const dashboardLink = screen.getByText('仪表盘').closest('a');
    expect(dashboardLink?.className).not.toContain('bg-sidebar-primary');
  });
});
