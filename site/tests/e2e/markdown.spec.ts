import { expect, test } from "@playwright/test";

test.describe("markdown docs for agents", () => {
  test.skip(({ isMobile }) => isMobile, "plain text is the same on every device");

  test("llms.txt links every guide and component page", async ({ request }) => {
    const index = await (await request.get("llms.txt")).text();
    expect(index).toContain("# iced-cube");
    const links = [...index.matchAll(/\]\((https:\/\/[^)]+\.md)\)/g)].map((match) => match[1] ?? "");
    expect(links.length).toBeGreaterThan(20);

    for (const link of links) {
      const path = new URL(link).pathname.replace(/^\/iced-cube\//, "");
      const response = await request.get(path);
      expect(response.status(), path).toBe(200);
      const body = await response.text();
      expect(body.startsWith("# "), path).toBe(true);
    }
  });

  test("component markdown carries code examples and no markup", async ({ request }) => {
    const page = await (await request.get("docs/components/button.md")).text();
    expect(page).toContain("## Examples");
    expect(page).toContain("```rust");
    expect(page).toContain("pub fn view(&self)");
    expect(page).not.toMatch(/<(div|span|iframe|button|img|script|style)\b/);
  });

  test("guide previews become code examples", async ({ request }) => {
    const theming = await (await request.get("docs/theming.md")).text();
    expect(theming).not.toContain("<Preview");
    expect(theming).not.toMatch(/^import /m);
    expect(theming).toContain("Light, dark and custom:");
    expect(theming).toContain("```rust");
  });

  test("llms-full contains every page", async ({ request }) => {
    const index = await (await request.get("llms.txt")).text();
    const full = await (await request.get("llms-full.txt")).text();
    for (const [, title] of index.matchAll(/^- \[([^\]]+)\]/gm)) {
      expect(full).toContain(`# ${title}`);
    }
  });

  test("the introduction links to the markdown docs", async ({ page }) => {
    await page.goto("docs/");
    await expect(page.getByRole("link", { name: "llms.txt" })).toHaveAttribute("href", /llms\.txt$/);
  });
});
