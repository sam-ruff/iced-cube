// Captures review screenshots of key pages: node tests/e2e/screenshots.mjs <base-url> <out-dir>
import { chromium, devices } from "@playwright/test";

const [base = "http://localhost:4322/iced-cube/", out = "screenshots"] = process.argv.slice(2);
const pages = ["", "docs/", "docs/components/", "docs/components/button/"];
const browser = await chromium.launch({ args: ["--enable-unsafe-swiftshader", "--use-angle=swiftshader"] });

for (const [name, options] of [
  ["desktop", { viewport: { width: 1440, height: 900 } }],
  ["mobile", devices["Pixel 7"]],
]) {
  for (const theme of ["light", "dark"]) {
    const context = await browser.newContext({ ...options, colorScheme: theme });
    const page = await context.newPage();
    for (const path of pages) {
      await page.goto(base + path);
      await page.waitForTimeout(2500);
      const file = `${out}/${name}-${theme}-${path.replaceAll("/", "_") || "home"}.png`;
      await page.screenshot({ path: file, fullPage: false });
    }
    await context.close();
  }
}
await browser.close();
