import { expect, test } from "@playwright/test";

test("demo page boots the app with desktop downloads beside it", async ({ page }) => {
  await page.goto("demo/");

  const frame = page.locator("iframe#demo");
  await expect(frame).toHaveAttribute("src", /story=showcase%2Fdemo/);
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

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

test("the front page opens the demo in a new tab", async ({ page }) => {
  await page.goto("./");
  const link = page.getByRole("link", { name: /Open the demo/ });
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("href", /\/demo\/$/);
});
