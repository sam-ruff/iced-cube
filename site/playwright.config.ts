import { defineConfig, devices } from "@playwright/test";

const port = 4322;

export default defineConfig({
  testDir: "tests/e2e",
  // Generous limits so that many browsers booting wasm at once on a
  // software renderer still pass. No retries: a flaky test is a bug.
  timeout: 240_000,
  expect: { timeout: 30_000 },
  fullyParallel: true,
  retries: 0,
  reporter: process.env["CI"] ? "line" : "list",
  use: {
    baseURL: `http://localhost:${port}/iced-cube/`,
    trace: "retain-on-failure",
    launchOptions: { args: ["--enable-unsafe-swiftshader", "--use-angle=swiftshader"] },
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } },
    { name: "mobile", use: { ...devices["Pixel 7"] } },
  ],
  webServer: {
    command: `npx astro preview --ignore-lock --port ${port}`,
    url: `http://localhost:${port}/iced-cube/`,
    reuseExistingServer: !process.env["CI"],
  },
});
