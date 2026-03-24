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

import type { Registry, Resource, ResourceLink, ResourceVersion, SkillFrontmatter, SkillFrontmatterData } from '@/types/v2';

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

import type { RegistryPlugin, LibraryPlugin, McpServer } from '@/types/v2';

// Registry Plugin APIs
export async function listRegistryPlugins(registryId: string): Promise<RegistryPlugin[]> {
  return invoke<RegistryPlugin[]>('list_registry_plugins', { registryId });
}

export async function getRegistryPluginResources(pluginId: string): Promise<Resource[]> {
  return invoke<Resource[]>('get_registry_plugin_resources', { pluginId });
}

export async function getRegistryPluginMcpServers(pluginId: string): Promise<McpServer[]> {
  return invoke<McpServer[]>('get_registry_plugin_mcp_servers', { pluginId });
}

export async function installPluginToProject(pluginId: string, projectId: string): Promise<unknown> {
  return invoke('install_plugin_to_project', { pluginId, projectId });
}

export async function installPluginToGlobal(pluginId: string): Promise<ResourceLink[]> {
  return invoke<ResourceLink[]>('install_plugin_to_global', { pluginId });
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
