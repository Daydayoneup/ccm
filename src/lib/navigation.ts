import type { NavigateFunction } from 'react-router-dom';
import { getResource } from '@/lib/tauri-api';
import type { Resource } from '@/types/v2';

/**
 * Navigate to the editor for a resource.
 * If the resource was installed from a library resource, navigates to the
 * library source instead (so edits go to the canonical copy).
 * Skills get extra query params for the skill editor mode.
 */
export async function navigateToResource(
  navigate: NavigateFunction,
  resource: Resource,
) {
  let target = resource;

  // If installed from library, redirect to the source resource
  if (resource.installed_from_id) {
    const source = await getResource(resource.installed_from_id);
    if (source) {
      target = source;
    }
  }

  const extra = target.resource_type === 'skill'
    ? `&resource_id=${target.id}&type=skill&scope=${target.scope}`
    : '';
  navigate(`/editor?file=${encodeURIComponent(target.source_path)}${extra}`);
}
