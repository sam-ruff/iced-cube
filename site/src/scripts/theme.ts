export type Theme = "light" | "dark";

export function currentTheme(): Theme {
  return document.documentElement.dataset["theme"] === "dark" ? "dark" : "light";
}

function store(theme: Theme): void {
  try {
    localStorage.setItem("theme", theme);
  } catch {
    // Storage can be unavailable in private windows; the choice then lasts for this page only.
  }
}

function apply(theme: Theme): void {
  document.documentElement.dataset["theme"] = theme;
  for (const button of document.querySelectorAll<HTMLButtonElement>("[data-theme-toggle]")) {
    button.setAttribute("aria-label", theme === "dark" ? "Switch to light theme" : "Switch to dark theme");
  }
  window.dispatchEvent(new CustomEvent<Theme>("themechange", { detail: theme }));
}

apply(currentTheme());

for (const button of document.querySelectorAll<HTMLButtonElement>("[data-theme-toggle]")) {
  button.addEventListener("click", () => {
    const next = currentTheme() === "dark" ? "light" : "dark";
    store(next);
    apply(next);
  });
}

const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
for (const kbd of document.querySelectorAll<HTMLElement>("[data-shortcut]")) {
  kbd.textContent = isMac ? "⌘K" : "Ctrl K";
}
