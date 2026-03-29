import { invoke } from '@tauri-apps/api/core';

// --- Types used by this module ---

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  is_symlink: boolean;
  size: number;
}

// --- Symlink Commands ---

/** Create a resource link (symlink) from source to target. */
export async function linkResource(source: string, target: string): Promise<void> {
  return invoke<void>('link_resource', { source, target });
}

/** Remove a resource link (symlink) at the target path. */
export async function unlinkResource(target: string): Promise<void> {
  return invoke<void>('unlink_resource', { target });
}

/** Check if a symlink at the target path is valid. */
export async function isSymlinkValid(target: string): Promise<boolean> {
  return invoke<boolean>('is_symlink_valid', { target });
}

// --- File Commands ---

/** Check if a path is a directory. */
export async function pathIsDirectory(path: string): Promise<boolean> {
  return invoke<boolean>('path_is_directory', { path });
}

/** Read file contents as a string. */
export async function readFile(path: string): Promise<string> {
  return invoke<string>('read_file', { path });
}

/** Write content to a file, creating parent directories if needed. */
export async function writeFile(path: string, content: string): Promise<void> {
  return invoke<void>('write_file', { path, content });
}

/** Delete a file or directory. */
export async function deletePath(path: string): Promise<void> {
  return invoke<void>('delete_path', { path });
}

/** Create a directory (and all parent directories). */
export async function createDirectory(path: string): Promise<void> {
  return invoke<void>('create_directory', { path });
}

/** List directory contents. */
export async function listDirectory(path: string): Promise<FileEntry[]> {
  return invoke<FileEntry[]>('list_directory', { path });
}

/** Compute SHA256 hash of a file. */
export async function fileContentHash(path: string): Promise<string> {
  return invoke<string>('file_content_hash', { path });
}

// --- Registry Commands ---

import type { DashboardStats, DiscoveredProject, InstalledResourceInfo, LibraryResourceWithInstalls, LinkType, MergedEnvVar, Plugin, Project, Registry, Resource, ResourceLink, ResourceType, ResourceVersion, SkillFrontmatter, SkillFrontmatterData } from '@/types/v2';

/** List all registries */
export async function listRegistries(): Promise<Registry[]> {
  return invoke<Registry[]>('list_registries');
}

/** Add a new registry by cloning a git repo */
export async function addRegistry(name: string, url: string, readonly: boolean): Promise<Registry> {
  return invoke<Registry>('add_registry', { name, url, readonly });
}

/** Remove a registry and its local clone */
export async function removeRegistry(id: string): Promise<void> {
  return invoke<void>('remove_registry', { id });
}

/** Sync (pull) a single registry */
export async function syncRegistry(id: string): Promise<Registry> {
  return invoke<Registry>('sync_registry', { id });
}

/** Sync all registries */
export async function syncAllRegistries(): Promise<Registry[]> {
  return invoke<Registry[]>('sync_all_registries');
}

/** Push changes for a private registry */
export async function pushRegistry(id: string, message: string): Promise<Registry> {
  return invoke<Registry>('push_registry', { id, message });
}

/** Check all registries for remote/local changes */
export async function checkRegistryUpdates(): Promise<Registry[]> {
  return invoke<Registry[]>('check_registry_updates');
}

/** List resources for a specific registry */
export async function listRegistryResources(registryId: string, resourceType?: string): Promise<Resource[]> {
  return invoke<Resource[]>('list_registry_resources', { registryId, resourceType: resourceType ?? null });
}

/** Publish a local resource to a private registry */
export async function publishToRegistry(resourceId: string, registryId: string): Promise<Resource> {
  return invoke<Resource>('publish_to_registry', { resourceId, registryId });
}

/** Install a registry resource to a project via symlink */
export async function installFromRegistry(resourceId: string, projectId: string): Promise<ResourceLink> {
  return invoke<ResourceLink>('install_from_registry', { resourceId, projectId });
}

/** Deploy a registry resource to global scope via symlink */
export async function deployFromRegistry(resourceId: string): Promise<ResourceLink> {
  return invoke<ResourceLink>('deploy_from_registry', { resourceId });
}

// --- Registry Plugin & Library Plugin APIs ---

import type { RegistryPlugin, LibraryPlugin } from '@/types/v2';

// Registry Plugin APIs
export async function listRegistryPlugins(registryId: string): Promise<RegistryPlugin[]> {
  return invoke<RegistryPlugin[]>('list_registry_plugins', { registryId });
}

export async function getRegistryPluginResources(pluginId: string): Promise<Resource[]> {
  return invoke<Resource[]>('get_registry_plugin_resources', { pluginId });
}

export async function installPluginToProject(pluginId: string, projectId: string): Promise<unknown> {
  return invoke('install_plugin_to_project', { pluginId, projectId });
}

export async function installPluginToGlobal(pluginId: string): Promise<ResourceLink[]> {
  return invoke<ResourceLink[]>('install_plugin_to_global', { pluginId });
}

export async function installResourceToProject(resourceId: string, projectId: string): Promise<ResourceLink[]> {
  return invoke<ResourceLink[]>('install_resource_to_project', { resourceId, projectId });
}

export async function installResourceToGlobal(resourceId: string): Promise<ResourceLink[]> {
  return invoke<ResourceLink[]>('install_resource_to_global', { resourceId });
}

export async function uninstallResource(linkIds: string[]): Promise<string[]> {
  return invoke<string[]>('uninstall_resource', { linkIds });
}

export async function getPluginResourcesInstallStatus(pluginId: string): Promise<Record<string, ResourceLink[]>> {
  return invoke<Record<string, ResourceLink[]>>('get_plugin_resources_install_status', { pluginId });
}

// Library Plugin APIs
export async function listLibraryPlugins(): Promise<LibraryPlugin[]> {
  return invoke<LibraryPlugin[]>('list_library_plugins');
}

export async function createLibraryPlugin(name: string, description: string | null, category: string | null): Promise<LibraryPlugin> {
  return invoke<LibraryPlugin>('create_library_plugin', { name, description, category });
}

export async function deleteLibraryPlugin(id: string): Promise<void> {
  return invoke<void>('delete_library_plugin', { id });
}

export async function addResourceToLibraryPlugin(pluginId: string, resourceId: string): Promise<void> {
  return invoke<void>('add_resource_to_library_plugin', { pluginId, resourceId });
}

export async function removeResourceFromLibraryPlugin(pluginId: string, resourceId: string): Promise<void> {
  return invoke<void>('remove_resource_from_library_plugin', { pluginId, resourceId });
}

export async function getLibraryPluginResources(pluginId: string): Promise<Resource[]> {
  return invoke<Resource[]>('get_library_plugin_resources', { pluginId });
}

// --- Resource Commands ---

/** Fetch a single resource by ID. */
export async function getResource(id: string): Promise<Resource | null> {
  return invoke<Resource | null>('get_resource', { id });
}

// --- Version Management Commands ---

/** Publish a new version for a resource. */
export async function publishResourceVersion(resourceId: string, version: string, changelog?: string): Promise<ResourceVersion> {
  return invoke<ResourceVersion>('publish_resource_version', { resourceId, version, changelog });
}

/** List all versions for a resource. */
export async function listResourceVersions(resourceId: string): Promise<ResourceVersion[]> {
  return invoke<ResourceVersion[]>('list_resource_versions', { resourceId });
}

/** Roll back a resource to a specific version. */
export async function rollbackResourceVersion(resourceId: string, version: string): Promise<void> {
  return invoke<void>('rollback_resource_version', { resourceId, version });
}

// --- Frontmatter Commands ---

/** Parse YAML frontmatter from a skill file. */
export async function parseSkillFrontmatter(filePath: string): Promise<SkillFrontmatterData> {
  return invoke<SkillFrontmatterData>('parse_skill_frontmatter', { filePath });
}

/** Save a skill file with updated frontmatter and body. */
export async function saveSkillWithFrontmatter(
  resourceId: string, filePath: string, frontmatterData: SkillFrontmatter, body: string
): Promise<void> {
  return invoke<void>('save_skill_with_frontmatter', { resourceId, filePath, frontmatterData, body });
}

/** Save raw skill file content (frontmatter + body). Backend parses frontmatter best-effort. */
export async function saveSkillRawContent(
  resourceId: string,
  filePath: string,
  content: string,
): Promise<void> {
  return invoke<void>('save_skill_raw_content', { resourceId, filePath, content });
}

/** Rename a file or directory from oldPath to newPath. */
export async function renamePath(oldPath: string, newPath: string): Promise<void> {
  await invoke('rename_path', { oldPath, newPath });
}

/** Fork a library resource to create an independent copy, optionally with a new name. */
export async function forkToLibrary(resourceId: string, newName?: string): Promise<Resource> {
  return invoke<Resource>('fork_to_library', { resourceId, newName: newName ?? null });
}

/** Update an installed resource to the latest registry version. */
export async function updateInstalledResource(resourceId: string): Promise<void> {
  return invoke<void>('update_installed_resource', { resourceId });
}

/** Retain a removed registry resource as a local library resource. */
export async function retainAsLibrary(resourceId: string): Promise<Resource> {
  return invoke<Resource>('retain_as_library', { resourceId });
}

/** List all installed resources with their links and registry info. */
export async function listInstalledResources(): Promise<InstalledResourceInfo[]> {
  return invoke<InstalledResourceInfo[]>('list_installed_resources');
}

/** Compute the content hash for an installed resource file. */
export async function computeInstalledHash(resourceType: string, resourceName: string): Promise<string | null> {
  return invoke<string | null>('compute_installed_hash', { resourceType, resourceName });
}

// --- Environment Variables ---

export async function listMergedEnvVars(projectId: string): Promise<MergedEnvVar[]> {
  return invoke<MergedEnvVar[]>('list_merged_env_vars', { projectId });
}

export async function setEnvVar(projectId: string | null, key: string, value: string): Promise<void> {
  return invoke<void>('set_env_var', { projectId, key, value });
}

export async function deleteEnvVar(id: string): Promise<void> {
  return invoke<void>('delete_env_var', { id });
}

// --- Terminal ---

export async function launchClaudeInTerminal(projectPath: string, terminal?: string): Promise<void> {
  return invoke<void>('launch_claude_in_terminal', { projectPath, terminal: terminal ?? null });
}

export async function getTerminalPreference(): Promise<string | null> {
  return invoke<string | null>('get_terminal_preference');
}

export async function setTerminalPreference(terminal: string): Promise<void> {
  return invoke<void>('set_terminal_preference', { terminal });
}

// --- App Settings ---

export async function getAppSetting(key: string): Promise<string | null> {
  return invoke<string | null>('get_app_setting', { key });
}

export async function setAppSetting(key: string, value: string): Promise<void> {
  return invoke<void>('set_app_setting', { key, value });
}

// --- MCP Servers ---

export async function updateMcpServerConfig(resourceId: string, newConfigJson: string): Promise<Resource> {
  return invoke<Resource>('update_mcp_server_config', { resourceId, newConfigJson });
}

export async function createMcpServer(projectId: string, name: string, configJson: string): Promise<Resource> {
  return invoke<Resource>('create_mcp_server', { projectId, name, configJson });
}

// --- Permissions ---

export async function updateProjectPermissions(projectId: string, allow: string[], deny: string[]): Promise<void> {
  return invoke<void>('update_project_permissions', { projectId, allow, deny });
}

// --- Sync ---

export async function fullSync(): Promise<void> {
  return invoke<void>('full_sync');
}

export async function syncScope(scope: string): Promise<void> {
  return invoke<void>('sync_scope', { scope });
}

// --- Proxy ---

export interface ProxyConfig {
  enabled: boolean;
  proxy_type: string;
  host: string;
  port: string;
  username: string | null;
  password: string | null;
}

export async function getProxyConfig(): Promise<ProxyConfig> {
  return invoke<ProxyConfig>('get_proxy_config');
}

export async function saveProxyConfig(config: ProxyConfig): Promise<void> {
  return invoke<void>('save_proxy_config', { config });
}

export async function testProxy(config: unknown): Promise<string> {
  return invoke<string>('test_proxy', { config });
}

// --- Permissions (read) ---

export async function getProjectPermissions(projectId: string): Promise<{ allow: string[]; deny: string[] }> {
  return invoke<{ allow: string[]; deny: string[] }>('get_project_permissions', { projectId });
}

// --- Project Commands ---

export async function listProjectsV2(): Promise<Project[]> {
  return invoke<Project[]>('list_projects_v2');
}

export async function registerProjectV2(path: string): Promise<Project> {
  return invoke<Project>('register_project_v2', { path });
}

export async function removeProjectV2(id: string, deleteFromDisk: boolean): Promise<{ warnings: string[] }> {
  return invoke<{ warnings: string[] }>('remove_project_v2', { id, deleteFromDisk });
}

export async function scanAndDiscoverProjects(directories: string[]): Promise<Project[]> {
  return invoke<Project[]>('scan_and_discover_projects', { directories });
}

export async function discoverClaudeProjects(): Promise<DiscoveredProject[]> {
  return invoke<DiscoveredProject[]>('discover_claude_projects');
}

export async function listProjectResources(projectId: string, resourceType?: string): Promise<Resource[]> {
  return invoke<Resource[]>('list_project_resources', { projectId, resourceType: resourceType ?? null });
}

export async function createProjectResource(projectId: string, resourceType: ResourceType, name: string, content: string): Promise<Resource> {
  return invoke<Resource>('create_project_resource', { projectId, resourceType, name, content });
}

export async function deleteProjectResource(resourceId: string): Promise<void> {
  return invoke<void>('delete_project_resource', { resourceId });
}

export async function publishToLibrary(resourceId: string, replaceWithSymlink: boolean): Promise<Resource> {
  return invoke<Resource>('publish_to_library', { resourceId, replaceWithSymlink });
}

export async function installFromLibrary(libraryResourceId: string, projectId: string, linkType: string): Promise<void> {
  return invoke<void>('install_from_library', { libraryResourceId, projectId, linkType });
}

export async function rescanProject(projectId: string): Promise<{ added: number; removed: number }> {
  return invoke<{ added: number; removed: number }>('rescan_project', { projectId });
}

export async function toggleProjectPin(projectId: string): Promise<Project> {
  return invoke<Project>('toggle_project_pin', { projectId });
}

// --- Global Resource Commands ---

export async function listGlobalResources(resourceType?: string): Promise<Resource[]> {
  return invoke<Resource[]>('list_global_resources', { resourceType: resourceType ?? null });
}

export async function createGlobalResource(resourceType: ResourceType, name: string, content: string): Promise<Resource> {
  return invoke<Resource>('create_global_resource', { resourceType, name, content });
}

export async function deleteGlobalResource(id: string, deleteFromDisk: boolean): Promise<void> {
  return invoke<void>('delete_global_resource', { id, deleteFromDisk });
}

export async function backupToLibrary(resourceId: string, replaceWithLink: boolean): Promise<Resource> {
  return invoke<Resource>('backup_to_library', { resourceId, replaceWithLink });
}

// --- Env Vars (raw list) ---

export async function listEnvVars(projectId: string | null): Promise<MergedEnvVar[]> {
  return invoke<MergedEnvVar[]>('list_env_vars', { projectId });
}

// --- API Server ---

export async function toggleApiServer(enabled: boolean): Promise<void> {
  return invoke<void>('toggle_api_server', { enabled });
}

export async function getApiTokenStatus(): Promise<string | null> {
  return invoke<string | null>('get_api_token_status');
}

export async function generateApiToken(): Promise<string> {
  return invoke<string>('generate_api_token');
}

// --- Library Resource Commands ---

export async function listLibraryResourcesWithInstalls(resourceType?: string): Promise<LibraryResourceWithInstalls[]> {
  return invoke<LibraryResourceWithInstalls[]>('list_library_resources_with_installs', { resourceType: resourceType ?? null });
}

export async function createLibraryResource(resourceType: ResourceType, name: string, description: string, content: string): Promise<Resource> {
  return invoke<Resource>('create_library_resource', { resourceType, name, description, content });
}

export async function deleteLibraryResource(id: string, deleteFromDisk: boolean): Promise<void> {
  return invoke<void>('delete_library_resource', { id, deleteFromDisk });
}

export async function installToProject(resourceId: string, projectId: string, linkType: LinkType): Promise<void> {
  return invoke<void>('install_to_project', { resourceId, projectId, linkType });
}

export async function deployToGlobal(resourceId: string, linkType: LinkType): Promise<void> {
  return invoke<void>('deploy_to_global', { resourceId, linkType });
}

export async function listResourceLinks(resourceId: string): Promise<ResourceLink[]> {
  return invoke<ResourceLink[]>('list_resource_links', { resourceId });
}

export async function checkLinkHealth(): Promise<{ link_id: string; target_path: string; healthy: boolean; error: string | null }[]> {
  return invoke<{ link_id: string; target_path: string; healthy: boolean; error: string | null }[]>('check_link_health', {});
}

// --- Plugin Commands ---

export async function listPluginsV2(): Promise<Plugin[]> {
  return invoke<Plugin[]>('list_plugins_v2');
}

export async function scanPlugins(): Promise<Plugin[]> {
  return invoke<Plugin[]>('scan_plugins');
}

export async function getPluginResources(pluginId: string, resourceType?: string): Promise<Resource[]> {
  return invoke<Resource[]>('get_plugin_resources', { pluginId, resourceType: resourceType ?? null });
}

export async function extractToLibrary(resourceId: string): Promise<Resource> {
  return invoke<Resource>('extract_to_library', { resourceId });
}

// --- Dashboard Commands ---

export async function getDashboardStats(): Promise<DashboardStats> {
  return invoke<DashboardStats>('get_dashboard_stats');
}

export async function getRecentResources(limit: number): Promise<Resource[]> {
  return invoke<Resource[]>('get_recent_resources', { limit });
}

export async function searchResources(query: string): Promise<Resource[]> {
  return invoke<Resource[]>('search_resources', { query });
}

// --- Ranked Project List ---

export async function listProjectsRanked(): Promise<Project[]> {
  return invoke<Project[]>('list_projects_ranked');
}

// --- Settings File ---

export async function readSettingsFile(path: string): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('read_settings_file', { path });
}

export async function writeSettingsFile(path: string, content: Record<string, unknown>): Promise<void> {
  return invoke<void>('write_settings_file', { path, content });
}
