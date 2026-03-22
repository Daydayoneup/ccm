export type ResourceType = 'skill' | 'agent' | 'rule' | 'hook' | 'mcp_server' | 'command';
export type ResourceScope = 'global' | 'library' | 'project' | 'plugin' | 'registry';
export type SyncStatus = 'synced' | 'modified' | 'deleted' | 'conflict';
export type LinkType = 'symlink' | 'copy' | 'config_merge' | 'plugin_install';

export interface Resource {
  id: string;
  resource_type: ResourceType;
  name: string;
  description: string | null;
  scope: ResourceScope;
  source_path: string;
  content_hash: string | null;
  metadata: string | null;
  created_at: string;
  updated_at: string;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  language: string | null;
  last_scanned: string | null;
  pinned: number;
  launch_count: number;
}

export interface Plugin {
  id: string;
  name: string;
  version: string | null;
  scope: string | null;
  install_path: string | null;
  status: 'installed' | 'available';
  last_checked: string | null;
}

export interface McpServer {
  id: string;
  name: string;
  project_id: string | null;
  server_type: string | null;
  command: string | null;
  args: string | null;
  url: string | null;
  env: string | null;
  source_path: string;
  registry_plugin_id: string | null;
}

export interface ResourceLink {
  id: string;
  resource_id: string;
  target_scope: 'global' | 'project';
  target_path: string;
  config_key: string | null;
  project_id: string | null;
  link_type: LinkType;
  created_at: string;
}

export interface SyncState {
  id: string;
  watched_path: string;
  last_hash: string | null;
  last_synced: string | null;
  status: SyncStatus;
}

export interface DashboardStats {
  global_count: number;
  project_count: number;
  plugin_count: number;
  library_count: number;
  registry_count: number;
}

export interface CreateResourceInput {
  resource_type: ResourceType;
  name: string;
  description?: string;
  content: string;
}

export interface DiscoveredProject {
  path: string;
  name: string;
  has_claude_config: boolean;
}

export interface EnvVar {
  id: string;
  project_id: string | null;
  key: string;
  value: string;
}

export interface MergedEnvVar {
  id: string;
  key: string;
  value: string;
  scope: 'global' | 'project';
}

export interface Registry {
  id: string;
  name: string;
  url: string;
  local_path: string;
  readonly: boolean;
  last_synced: string | null;
  has_remote_changes: boolean;
  has_local_changes: boolean;
  created_at: string;
}

export interface RegistryPlugin {
  id: string;
  registry_id: string;
  name: string;
  description: string | null;
  category: string | null;
  source_path: string;
  source_type: 'local' | 'external';
  source_url: string | null;
  homepage: string | null;
}

export interface LibraryPlugin {
  id: string;
  name: string;
  description: string | null;
  category: string | null;
  created_at: string;
  updated_at: string;
}
