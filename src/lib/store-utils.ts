/**
 * Wraps an async store action with loading/error state management.
 *
 * Sets `[loadingKey]: true` and `error: null` before execution,
 * then `[loadingKey]: false` after (success or failure).
 * On error, sets `error` to the stringified error message.
 *
 * Returns the result on success, or `undefined` on error.
 */
export async function asyncAction<T>(
  set: (partial: Record<string, unknown>) => void,
  loadingKey: string,
  fn: () => Promise<T>,
): Promise<T | undefined> {
  set({ [loadingKey]: true, error: null });
  try {
    const result = await fn();
    set({ [loadingKey]: false });
    return result;
  } catch (e) {
    set({ error: String(e), [loadingKey]: false });
    return undefined;
  }
}
