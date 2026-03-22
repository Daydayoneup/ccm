import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { CommandPalette } from '../CommandPalette';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/stores/project-store-v2', () => ({
  useProjectStoreV2: () => ({
    togglePin: vi.fn(),
  }),
}));

import { invoke } from '@tauri-apps/api/core';
const mockInvoke = vi.mocked(invoke);

const mockProjects = [
  { id: '1', name: 'alpha', path: '/home/user/alpha', language: 'Rust', last_scanned: null, pinned: 1, launch_count: 5 },
  { id: '2', name: 'bravo', path: '/home/user/bravo', language: 'Go', last_scanned: null, pinned: 0, launch_count: 10 },
  { id: '3', name: 'charlie', path: '/home/user/charlie', language: null, last_scanned: null, pinned: 0, launch_count: 1 },
];

function renderPalette(open = true) {
  const onClose = vi.fn();
  const result = render(
    <MemoryRouter>
      <CommandPalette open={open} onClose={onClose} />
    </MemoryRouter>
  );
  return { ...result, onClose };
}

describe('CommandPalette', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === 'list_projects_ranked') return Promise.resolve(mockProjects);
      if (cmd === 'launch_claude_in_terminal') return Promise.resolve();
      return Promise.resolve(null);
    });
  });

  it('renders nothing when closed', () => {
    renderPalette(false);
    expect(screen.queryByPlaceholderText('搜索项目...')).toBeNull();
  });

  it('renders search input when open', async () => {
    renderPalette(true);
    expect(await screen.findByPlaceholderText('搜索项目...')).toBeInTheDocument();
  });

  it('loads and displays projects', async () => {
    renderPalette(true);
    expect(await screen.findByText('alpha')).toBeInTheDocument();
    expect(screen.getByText('bravo')).toBeInTheDocument();
    expect(screen.getByText('charlie')).toBeInTheDocument();
  });

  it('shows Pinned and Recent sections', async () => {
    renderPalette(true);
    expect(await screen.findByText('Pinned')).toBeInTheDocument();
    expect(screen.getByText('Recent')).toBeInTheDocument();
  });

  it('filters projects by name query', async () => {
    renderPalette(true);
    await screen.findByText('alpha');
    const input = screen.getByPlaceholderText('搜索项目...');
    fireEvent.change(input, { target: { value: 'bra' } });
    expect(screen.getByText('bravo')).toBeInTheDocument();
    expect(screen.queryByText('alpha')).toBeNull();
    expect(screen.queryByText('charlie')).toBeNull();
  });

  it('filters projects by path query (case insensitive)', async () => {
    renderPalette(true);
    await screen.findByText('alpha');
    const input = screen.getByPlaceholderText('搜索项目...');
    fireEvent.change(input, { target: { value: '/HOME/USER/CHARLIE' } });
    expect(screen.getByText('charlie')).toBeInTheDocument();
    expect(screen.queryByText('alpha')).toBeNull();
  });

  it('shows empty message when no match', async () => {
    renderPalette(true);
    await screen.findByText('alpha');
    const input = screen.getByPlaceholderText('搜索项目...');
    fireEvent.change(input, { target: { value: 'zzzzz' } });
    expect(screen.getByText('No matching projects')).toBeInTheDocument();
  });

  it('closes on Escape key', async () => {
    const { onClose } = renderPalette(true);
    await screen.findByText('alpha');
    fireEvent.keyDown(screen.getByPlaceholderText('搜索项目...'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  it('closes when clicking overlay', async () => {
    const { onClose, container } = renderPalette(true);
    await screen.findByText('alpha');
    const overlay = container.querySelector('.fixed.inset-0');
    if (overlay) fireEvent.click(overlay);
    expect(onClose).toHaveBeenCalled();
  });
});
