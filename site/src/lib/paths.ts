const base = import.meta.env.BASE_URL.replace(/\/$/, "");

/** Prefixes a site-absolute path with the deployment base. */
export function href(path: string): string {
  return `${base}${path.startsWith("/") ? path : `/${path}`}`;
}
