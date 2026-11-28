import { expect, type Locator, type Page } from "@playwright/test";

/** Several wasm previews booting at once on a software renderer are slow. */
export const BOOT_MS = 120_000;
/** How long a preview may take to handle input and draw it on a loaded machine. */
export const HANDLED_MS = 60_000;

let pings = 0;

/** Opens `path` with previews that report their state. */
export async function visit(page: Page, path: string): Promise<void> {
  await page.goto(`${path}?preview-state`);
}

/** Scrolls a story's frame into view and waits for its app to draw its first state. */
export async function live(page: Page, story: string): Promise<Locator> {
  const frame = page.locator(`[data-story="${story}"]`).first();
  await frame.scrollIntoViewIfNeeded();
  await expect(frame).toHaveClass(/live/, { timeout: BOOT_MS });
  await expect(frame).toHaveAttribute("data-state", /./, { timeout: BOOT_MS });
  return frame;
}

/**
 * Waits until the preview has handled everything sent to it so far and drawn
 * the result, by sending it a ping and waiting to hear it back. Returns the
 * story's state.
 */
export async function handled(frame: Locator): Promise<string> {
  const value = `${process.pid}-${(pings += 1)}`;
  await frame
    .locator("iframe")
    .evaluate((iframe, ping) => (iframe as HTMLIFrameElement).contentWindow?.postMessage({ type: "ping", value: ping }, "*"), value);
  await expect(frame).toHaveAttribute("data-ping", value, { timeout: HANDLED_MS });
  return (await frame.getAttribute("data-state")) ?? "";
}

/** What the frame looks like now. */
export async function picture(frame: Locator): Promise<string> {
  return (await frame.screenshot()).toString("base64");
}
