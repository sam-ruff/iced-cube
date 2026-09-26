import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://sam-ruff.github.io",
  base: "/iced-cube",
  trailingSlash: "always",
  markdown: {
    shikiConfig: {
      themes: { light: "github-light", dark: "github-dark-default" },
      defaultColor: false,
    },
  },
});
