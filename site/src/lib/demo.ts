/** The GitHub API entry for the newest release, which lists its assets. */
export const LATEST_RELEASE = "https://api.github.com/repos/sam-ruff/iced-cube/releases/latest";

export type Availability = "available" | "missing" | "unknown";

/**
 * Whether the latest release carries `file`, from the status and body of a
 * request for {@link LATEST_RELEASE}. No release at all counts as missing;
 * anything else unexpected, such as a rate limit, is unknown, so the link
 * stays as it is.
 */
export function assetAvailability(status: number, body: unknown, file: string): Availability {
  if (status === 404) return "missing";
  if (status !== 200 || typeof body !== "object" || body === null) return "unknown";
  const assets = (body as { assets?: unknown }).assets;
  if (!Array.isArray(assets)) return "unknown";
  const names = assets.map((asset: unknown) =>
    typeof asset === "object" && asset !== null ? (asset as { name?: unknown }).name : undefined,
  );
  return names.includes(file) ? "available" : "missing";
}

export type AppTheme = "light" | "dark" | "system";

/** The theme the demo app says it picked, from a posted message, if it is one. */
export function appTheme(data: unknown): AppTheme | null {
  if (typeof data !== "object" || data === null) return null;
  const { type, value } = data as { type?: unknown; value?: unknown };
  if (type !== "app-theme") return null;
  return value === "light" || value === "dark" || value === "system" ? value : null;
}
