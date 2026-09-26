/** The tallest a preview frame may grow to fit its story. */
export const MAX_FRAME_HEIGHT = 1200;

/**
 * The height of a preview frame once its story reports how tall it needs to
 * be. A frame never shrinks below the story's own height, so desktop layouts
 * keep their spacing, and it only grows when rows wrap on a narrow screen.
 */
export function frameHeight(base: number, reported: unknown): number {
  if (typeof reported !== "number" || !Number.isFinite(reported)) return base;
  return Math.min(MAX_FRAME_HEIGHT, Math.max(base, Math.ceil(reported)));
}
