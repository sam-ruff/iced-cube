/**
 * Decides which previews may hold a live iframe. Browsers cap the number of
 * GPU contexts per page, so only `capacity` previews run at once.
 */
export interface Plan<T> {
  mount: T[];
  unmount: T[];
}

/**
 * Plans mounts for `wanted` previews given those already `mounted`.
 * Mounted previews that are no longer wanted are evicted first, farthest
 * from the viewport first, as measured by `distance`.
 */
export function plan<T>(
  wanted: readonly T[],
  mounted: readonly T[],
  capacity: number,
  distance: (item: T) => number,
): Plan<T> {
  const mountedSet = new Set(mounted);
  const wantedSet = new Set(wanted);
  const pending = wanted
    .filter((item) => !mountedSet.has(item))
    .sort((a, b) => distance(a) - distance(b));
  const evictable = mounted
    .filter((item) => !wantedSet.has(item))
    .sort((a, b) => distance(b) - distance(a));

  const mount: T[] = [];
  const unmount: T[] = [];
  let free = Math.max(0, capacity - mounted.length);

  for (const item of pending) {
    if (free === 0) {
      const victim = evictable.shift();
      if (victim === undefined) break;
      unmount.push(victim);
      free += 1;
    }
    mount.push(item);
    free -= 1;
  }

  return { mount, unmount };
}
