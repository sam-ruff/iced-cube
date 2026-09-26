import { expect, test } from "@playwright/test";
import { PNG } from "pngjs";

/** Leftmost non-background pixel as [x, y], scanning columns left to right. */
function firstDark(png: Buffer): [number, number] {
  const image = PNG.sync.read(png);
  for (let x = 0; x < image.width; x += 1) {
    for (let y = 0; y < image.height; y += 1) {
      const i = (y * image.width + x) * 4;
      if ((image.data[i] ?? 255) < 200) return [x, y];
    }
  }
  return [-1, -1];
}

// Previews are centred, so a story whose width changes with its state would
// shift sideways when clicked.
test("radio group stays put when the selection changes", async ({ page, isMobile }) => {
  test.skip(isMobile, "one viewport is enough for a layout check");
  await page.setViewportSize({ width: 720, height: 280 });
  await page.goto("preview/?story=radio%2Fdefault&theme=light");
  const canvas = page.locator("canvas");
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(500);

  const box = await canvas.boundingBox();
  if (!box) throw new Error("canvas has no bounding box");
  const before = await canvas.screenshot();
  const [left, top] = firstDark(before);
  expect(left).toBeGreaterThan(0);

  // The leftmost dark pixel is the edge of the first radio circle (16px wide).
  // iced learns the cursor position from move events, so hover before clicking.
  const target = { x: box.x + left + 8, y: box.y + top + 8 };
  await page.mouse.move(target.x - 4, target.y);
  await page.mouse.move(target.x, target.y);
  await page.waitForTimeout(100);
  await page.mouse.click(target.x, target.y);
  await page.waitForTimeout(300);
  const after = await canvas.screenshot();

  expect(after.equals(before), "the click should change the selection").toBe(false);
  expect(firstDark(after)[0]).toBe(left);
});
