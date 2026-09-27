import { type Page, expect, test } from "@playwright/test";

const LINUX = "iced-cube-demo-linux-x86_64.tar.gz";
const WINDOWS = "iced-cube-demo-windows-x86_64.zip";

/** Answers the page's release lookup without reaching GitHub. */
async function release(page: Page, status: number, assets: string[] = []) {
  await page.route("https://api.github.com/repos/sam-ruff/iced-cube/releases/latest", (route) =>
    route.fulfill({
      status,
      contentType: "application/json",
      body: JSON.stringify(status === 200 ? { assets: assets.map((name) => ({ name })) } : { message: "Not Found" }),
    }),
  );
}

test("demo page boots the app with desktop downloads beside it", async ({ page }) => {
  await release(page, 200, [LINUX, WINDOWS]);
  await page.goto("demo/");

  const frame = page.locator("iframe#demo");
  await expect(frame).toHaveAttribute("src", /story=showcase%2Fdemo/);
  await expect(frame).toHaveAttribute("allow", /clipboard-write/);
  await expect(page.frameLocator("iframe#demo").locator("canvas")).toBeVisible({ timeout: 30_000 });

  const downloads = page.getByRole("navigation", { name: "Desktop downloads" });
  await expect(downloads.getByRole("link", { name: "Download the demo for Linux" })).toHaveAttribute(
    "href",
    /iced-cube-demo-linux-x86_64\.tar\.gz$/,
  );
  await expect(downloads.getByRole("link", { name: "Download the demo for Windows" })).toHaveAttribute(
    "href",
    /iced-cube-demo-windows-x86_64\.zip$/,
  );
  await expect(page.locator("[data-download-note]")).toBeHidden();

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

test("downloads a release does not carry yet are disabled with a note", async ({ page }) => {
  await release(page, 404);
  await page.goto("demo/");

  const note = page.locator("[data-download-note]");
  await expect(note).toBeVisible();
  await expect(note).toHaveText("Arrives with the next release");
  for (const os of ["Linux", "Windows"]) {
    const link = page.locator(`a[data-os="${os}"]`);
    await expect(link).toHaveAttribute("aria-disabled", "true");
    await expect(link).not.toHaveAttribute("href", /.+/);
  }

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

test("a theme picked in the app carries over to the page", async ({ page }) => {
  await release(page, 200, [LINUX, WINDOWS]);
  await page.goto("demo/");
  await page.evaluate(() => {
    document.documentElement.dataset["theme"] = "light";
  });
  await expect(page.locator("iframe#demo")).toHaveAttribute("src", /preview/);
  const app = await (await page.locator("iframe#demo").elementHandle())?.contentFrame();
  expect(app).toBeTruthy();
  await app?.waitForLoadState();

  await app?.evaluate(() => window.parent.postMessage({ type: "app-theme", value: "dark" }, "*"));
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("the front page opens the demo in a new tab", async ({ page }) => {
  await page.goto("./");
  const link = page.getByRole("link", { name: /Open the demo/ });
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("href", /\/demo\/$/);
});
