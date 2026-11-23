import { expect, test, type Locator, type Page } from "@playwright/test";

const STORIES = [
  "calendar/month",
  "calendar/week",
  "calendar/agenda",
  "calendar/year",
  "calendar/create",
  "calendar/read-only",
  "calendar/repeating",
  "calendar/themed",
  "calendar/keyboard",
  "calendar/reminders",
  "calendar/missed",
  "calendar/triggers",
];

async function noHorizontalScroll(page: Page): Promise<void> {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
}

/** Scrolls a story's frame into view and waits for its wasm app to draw. */
async function live(page: Page, story: string): Promise<Locator> {
  const frame = page.locator(`[data-story="${story}"]`).first();
  await frame.scrollIntoViewIfNeeded();
  await expect(frame).toHaveClass(/live/, { timeout: 30_000 });
  return frame;
}

/** What the frame looks like now, for telling whether the app redrew. */
async function picture(frame: Locator): Promise<string> {
  return (await frame.screenshot()).toString("base64");
}

/** Waits until the frame has stopped changing, and returns how it looks. */
async function settled(frame: Locator): Promise<string> {
  let last = await picture(frame);
  await expect
    .poll(
      async () => {
        const now = await picture(frame);
        const same = now === last;
        last = now;
        return same;
      },
      { timeout: 15_000, intervals: [250] },
    )
    .toBe(true);
  return last;
}

/** Waits for the frame to look different from `before`, then to settle. */
async function changedFrom(frame: Locator, before: string): Promise<string> {
  await expect.poll(async () => (await picture(frame)) !== before, { timeout: 15_000 }).toBe(true);
  return settled(frame);
}

test("the calendar page shows every example, the form's API and the keymap", async ({ page }) => {
  await page.goto("docs/components/calendar/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Calendar");

  for (const story of STORIES) {
    await expect(page.locator(`[data-story="${story}"]`).first(), story).toBeAttached();
  }
  // Calendars scroll inside their previews.
  await expect(page.locator('[data-story="calendar/week"]').first()).toHaveAttribute("data-wheel", "true");

  for (const id of ["views", "the-entry-form", "repeating-entries", "triggers", "theming", "narrow-screens"]) {
    await expect(page.locator(`h2#${id}`), id).toBeVisible();
  }
  const main = page.locator("main");
  await expect(main).toContainText("editor::State::new(&draft, &time_zone)");
  await expect(main).toContainText("location and notes");
  await expect(main).toContainText("every two weeks");

  const keys = page.getByRole("region", { name: "Keyboard" });
  await expect(keys.locator("kbd", { hasText: "PageDown" })).toBeVisible();
  await expect(keys.locator("kbd", { hasText: "Escape" })).toBeVisible();
  await noHorizontalScroll(page);
});

test("the creating example shows the code of the entry form", async ({ page }) => {
  await page.goto("docs/components/calendar/");
  const root = page.locator("[data-preview-root]").filter({ has: page.locator('[data-story="calendar/create"]') });
  await root.getByRole("tab", { name: "Code" }).click();
  const code = root.locator('[data-panel="code"] pre');
  await expect(code).toContainText("editor::State::new(&draft, time_zone)");
  await expect(code).toContainText("calendar::Event::Insert(entry)");
  await expect(code).toContainText(".on_submit(Message::Save)");
  await noHorizontalScroll(page);
});

test("the calendar page links to the events guide and back", async ({ page }) => {
  await page.goto("docs/components/calendar/");
  await page.locator('a[href="../../calendar-events/"]').first().click();
  await expect(page).toHaveURL(/docs\/calendar-events\/$/);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Calendar events");
  for (const story of ["calendar/reminders", "calendar/missed", "calendar/triggers"]) {
    await expect(page.locator(`[data-story="${story}"]`).first(), story).toBeAttached();
  }
  await expect(page.locator("main")).toContainText("resume_from");
  await noHorizontalScroll(page);

  await page.locator('a[href="../components/calendar/"]').first().click();
  await expect(page).toHaveURL(/docs\/components\/calendar\/$/);
});

test("search finds the calendar by what people call it", async ({ page }) => {
  await page.goto("./");
  await page.locator("[data-search-open]").click();
  const input = page.locator("[data-search-input]");
  await input.fill("reminders");
  await expect(page.getByRole("option").filter({ hasText: "Calendar" }).first()).toBeVisible();
  // The component and its guide both answer to the name.
  await input.fill("calend");
  await expect(page.getByRole("option").first()).toContainText("Calendar");
  await expect(page.getByRole("option").filter({ hasText: "Calendar events" }).first()).toBeVisible();
});

test("the Markdown version of the calendar page carries the form's example and notes", async ({ request }) => {
  const response = await request.get("docs/components/calendar.md");
  expect(response.ok()).toBe(true);
  const body = await response.text();
  expect(body).toContain("# Calendar");
  expect(body).toContain("## The entry form");
  expect(body).toContain("editor::State::edit");
  expect(body).toContain("location_of()");
  expect(body).toContain("pub struct Example");
  expect(body).not.toContain("<iframe");
});

test("every calendar example boots in the browser", async ({ page }) => {
  test.slow();
  await page.goto("docs/components/calendar/");
  for (const story of STORIES) {
    const frame = await live(page, story);
    await expect(frame.locator("iframe"), story).toHaveAttribute("title", /Live preview/);
    const box = await frame.boundingBox();
    expect(box?.width ?? 0, story).toBeGreaterThan(200);
  }
  await noHorizontalScroll(page);
});

// The app draws to a canvas, so these read the preview as a picture: it
// changes when the form opens, and again when the entry is on the week.
test("dragging across the live week opens the entry form, and Enter saves it", async ({ page, isMobile }) => {
  test.skip(isMobile, "a finger scrolls the hours; dragging needs a mouse");
  await page.goto("docs/components/calendar/");
  const frame = await live(page, "calendar/create");
  const box = await frame.boundingBox();
  if (!box) throw new Error("no preview frame");
  const week = await settled(frame);

  // Down the right-hand side, clear of both entries, in Sunday's column.
  const x = box.x + box.width - 60;
  await page.mouse.move(x, box.y + 240);
  await page.mouse.down();
  await page.mouse.move(x, box.y + 270, { steps: 4 });
  await page.mouse.move(x, box.y + 300, { steps: 4 });
  await page.mouse.up();
  const form = await changedFrom(frame, week);

  // The dialog puts focus in the title.
  await page.keyboard.type("Retro");
  const typed = await changedFrom(frame, form);
  await page.keyboard.press("Enter");
  const saved = await changedFrom(frame, typed);
  expect(saved).not.toBe(week);
  expect(saved).not.toBe(form);

  // Restarting the preview puts the week back as it was.
  const root = page.locator("[data-preview-root]").filter({ has: frame });
  await root.locator("[data-reset]").click();
  await expect(frame).toHaveClass(/live/, { timeout: 30_000 });
  await expect.poll(async () => (await settled(frame)) === week, { timeout: 30_000 }).toBe(true);
});

test("Escape closes the entry form without adding anything", async ({ page, isMobile }) => {
  test.skip(isMobile, "needs a mouse and a keyboard");
  await page.goto("docs/components/calendar/");
  const frame = await live(page, "calendar/create");
  const box = await frame.boundingBox();
  if (!box) throw new Error("no preview frame");

  // Park the pointer where it will end up, so hover looks the same after.
  const x = box.x + box.width - 60;
  await page.mouse.move(x, box.y + 300);
  const week = await settled(frame);

  await page.mouse.move(x, box.y + 240);
  await page.mouse.down();
  await page.mouse.move(x, box.y + 300, { steps: 6 });
  await page.mouse.up();
  const form = await changedFrom(frame, week);

  // The first Escape leaves the title, the second closes the dialog.
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  const closed = await changedFrom(frame, form);
  expect(closed).not.toBe(form);
});

test("the header of a live calendar moves to the next month and Today comes back", async ({ page, isMobile }) => {
  test.skip(isMobile, "the header lays out differently on a phone");
  await page.goto("docs/components/calendar/");
  const frame = await live(page, "calendar/month");
  const box = await frame.boundingBox();
  if (!box) throw new Error("no preview frame");

  // Today is at the left of the header, with the arrows after it. Today
  // is pressed first, so the picture to come back to has the pointer on
  // the button and whatever Today selects.
  const today = { x: box.x + 60, y: box.y + 42 };
  const next = { x: box.x + 160, y: box.y + 42 };
  await page.mouse.click(today.x, today.y);
  const june = await settled(frame);

  await page.mouse.click(next.x, next.y);
  const july = await changedFrom(frame, june);

  await page.mouse.click(today.x, today.y);
  await changedFrom(frame, july);
  await expect.poll(async () => (await picture(frame)) === june, { timeout: 15_000 }).toBe(true);
});

test("the calendar pages fit a phone", async ({ page, isMobile }) => {
  test.skip(!isMobile, "phone only");
  for (const path of ["docs/components/calendar/", "docs/calendar-events/"]) {
    await page.goto(path);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await noHorizontalScroll(page);
  }
  await page.goto("docs/components/calendar/");
  const frame = await live(page, "calendar/month");
  const box = await frame.boundingBox();
  const viewport = page.viewportSize();
  expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(viewport?.width ?? 0);
});
