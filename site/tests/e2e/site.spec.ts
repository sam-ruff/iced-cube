import { expect, test, type Page } from "@playwright/test";

async function noHorizontalScroll(page: Page): Promise<void> {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
}

test("home page shows the hero, a live example and the component grid", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Components for iced");
  await expect(page.locator(".card").first()).toBeVisible();
  await noHorizontalScroll(page);
});

test("component page shows the preview tab first and switches to code", async ({ page }) => {
  await page.goto("docs/components/button/");
  const firstPreview = page.locator("[data-preview-root]").first();
  const previewTab = firstPreview.getByRole("tab", { name: "Preview" });
  const codeTab = firstPreview.getByRole("tab", { name: "Code" });

  await expect(previewTab).toHaveAttribute("aria-selected", "true");
  await expect(firstPreview.locator('[data-panel="code"]')).toBeHidden();

  await codeTab.click();
  await expect(codeTab).toHaveAttribute("aria-selected", "true");
  await expect(firstPreview.locator('[data-panel="code"] pre')).toContainText("fn view");
  await noHorizontalScroll(page);
});

test("live preview boots the wasm app and reports ready", async ({ page }) => {
  await page.goto("docs/components/button/");
  const frame = page.locator("[data-story]").first();
  await expect(frame).toHaveClass(/live/, { timeout: 30_000 });
  await expect(frame.locator("iframe")).toHaveAttribute("title", /Live preview/);
});

test("live previews never exceed the concurrency cap", async ({ page }) => {
  await page.goto("docs/components/button/");
  for (let y = 0; y < 6; y += 1) {
    await page.mouse.wheel(0, 600);
    await page.waitForTimeout(150);
    expect(await page.locator("[data-story] iframe").count()).toBeLessThanOrEqual(8);
  }
});

test("theme toggle switches the page and posters", async ({ page }) => {
  await page.goto("docs/components/button/");
  const html = page.locator("html");
  const before = await html.getAttribute("data-theme");
  await page.locator("[data-theme-toggle]").click();
  await expect(html).not.toHaveAttribute("data-theme", before ?? "");
  const theme = await html.getAttribute("data-theme");
  await expect(page.locator(`[data-story] img.poster.${theme}`).first()).toBeVisible();
  const other = theme === "dark" ? "light" : "dark";
  await expect(page.locator(`[data-story] img.poster.${other}`).first()).toBeHidden();
});

test("search finds a component and navigates to it", async ({ page }) => {
  await page.goto("./");
  await page.locator("[data-search-open]").click();
  const input = page.locator("[data-search-input]");
  await expect(input).toBeFocused();
  await input.fill("butt");
  await expect(page.getByRole("option").first()).toContainText("Button");
  await input.press("Enter");
  await expect(page).toHaveURL(/docs\/components\/button\/$/);
});

test("search finds a component by another name", async ({ page }) => {
  await page.goto("./");
  await page.locator("[data-search-open]").click();
  const input = page.locator("[data-search-input]");
  await input.fill("toggle");
  await expect(page.getByRole("option").first()).toContainText("Switch");
  await input.fill("picker");
  await expect(page.getByRole("option").first()).toContainText("Select");
  await input.press("Enter");
  await expect(page).toHaveURL(/docs\/components\/select\/$/);
});

test("component page links to related components", async ({ page }) => {
  await page.goto("docs/components/checkbox/");
  const related = page.locator("[data-related]");
  await expect(related).toContainText("See also");
  await related.getByRole("link", { name: "Switch" }).click();
  await expect(page).toHaveURL(/docs\/components\/switch\/$/);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Switch");
});

test("the last guide leads into the first component and back", async ({ page }) => {
  await page.goto("docs/status/");
  await page.locator(".pager .next").click();
  await expect(page).toHaveURL(/docs\/components\/button\/$/);
  await page.locator(".pager .prev").click();
  await expect(page).toHaveURL(/docs\/status\/$/);
});

test("keyboard shortcut opens search", async ({ page, isMobile }) => {
  test.skip(isMobile, "no hardware keyboard");
  await page.goto("docs/");
  await page.keyboard.press("Control+k");
  await expect(page.locator("[data-search]")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("[data-search]")).toBeHidden();
});

test("mobile navigation opens and closes", async ({ page, isMobile }) => {
  test.skip(!isMobile, "sidebar is always visible on desktop");
  await page.goto("docs/");
  const toggle = page.locator("[data-menu-toggle]");
  await toggle.click();
  await expect(page.locator("#sidebar")).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
});

test("unknown pages show the not found page", async ({ page }) => {
  const response = await page.goto("does-not-exist/");
  expect(response?.status()).toBe(404);
  await expect(page.getByRole("heading", { level: 1 })).toContainText("empty");
});

test("component keyboard table comes from the Rust keymap and links to the guide", async ({ page }) => {
  await page.goto("docs/components/tabs/");
  const table = page.getByRole("region", { name: "Keyboard" });
  await expect(table.locator("kbd", { hasText: "Ctrl+Shift+Tab" })).toBeVisible();
  await expect(table).toContainText("Selects the next enabled tab");
  await page.locator('section:has(#keyboard) a[href$="/docs/keyboard/"]').click();
  await expect(page).toHaveURL(/docs\/keyboard\/$/);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Keyboard shortcuts");
  await noHorizontalScroll(page);
});

test("preview frames use each story's height", async ({ page }) => {
  await page.goto("docs/components/button/");
  const short = page.locator('[data-story="button/variants"]').first();
  await expect(short).toHaveCSS("height", "160px");

  await page.goto("docs/components/toast/");
  const tall = page.locator('[data-story="toast/variants"]').first();
  await expect(tall).toHaveCSS("height", "340px");
});

// Stat cards sit in one row on desktop and stack on a phone, so the frame
// grows to the height the story reports.
test("a preview whose cards wrap grows to fit them", async ({ page, isMobile }) => {
  await page.goto("docs/components/card/");
  const frame = page.locator('[data-story="card/stats"]').first();
  await frame.scrollIntoViewIfNeeded();
  await expect(frame).toHaveClass(/live/, { timeout: 30_000 });

  if (!isMobile) {
    await expect(frame).toHaveCSS("height", "280px");
    return;
  }
  await expect
    .poll(async () => (await frame.boundingBox())?.height ?? 0, { timeout: 10_000 })
    .toBeGreaterThan(400);
});
