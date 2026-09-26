import { rank, type Entry } from "../lib/rank";

interface PagefindResult {
  url: string;
  meta: { title?: string };
  excerpt: string;
}

interface Pagefind {
  search(query: string): Promise<{ results: { data(): Promise<PagefindResult> }[] }>;
}

interface Result {
  title: string;
  detail: string;
  href: string;
  html?: boolean;
}

const dialog = document.querySelector<HTMLDialogElement>("[data-search]");
const input = document.querySelector<HTMLInputElement>("[data-search-input]");
const list = document.querySelector<HTMLUListElement>("[data-search-results]");
const empty = document.querySelector<HTMLElement>("[data-search-empty]");
const status = document.querySelector<HTMLElement>("[data-search-status]");

function readJson<T>(selector: string, fallback: T): T {
  const text = document.querySelector(selector)?.textContent;
  if (!text) return fallback;
  try {
    return JSON.parse(text) as T;
  } catch {
    return fallback;
  }
}

const entries = readJson<Entry[]>("[data-search-index]", []);
const pagefindUrl = readJson<string>("[data-pagefind-url]", "");

let pagefind: Pagefind | null | undefined;
let active = 0;
let generation = 0;
let optionCount = 0;
let groupCount = 0;
let returnFocus: HTMLElement | null = null;

async function loadPagefind(): Promise<Pagefind | null> {
  if (pagefind !== undefined) return pagefind;
  try {
    pagefind = (await import(/* @vite-ignore */ pagefindUrl)) as Pagefind;
  } catch {
    // The index only exists in production builds.
    pagefind = null;
  }
  return pagefind;
}

function option(result: Result): HTMLLIElement {
  const item = document.createElement("li");
  item.setAttribute("role", "option");
  item.setAttribute("aria-selected", "false");
  item.id = `search-option-${optionCount++}`;
  const link = document.createElement("a");
  link.href = result.href;
  link.tabIndex = -1;
  const strong = document.createElement("strong");
  strong.textContent = result.title;
  const span = document.createElement("span");
  if (result.html) span.innerHTML = result.detail;
  else span.textContent = result.detail;
  link.append(strong, span);
  item.append(link);
  item.addEventListener("mousemove", () => highlight(options().indexOf(item), false));
  return item;
}

/** Appends a labelled group of options to the listbox. */
function group(label: string, results: Result[]): void {
  if (!list || results.length === 0) return;
  const id = `search-group-${groupCount++}`;
  const item = document.createElement("li");
  item.setAttribute("role", "group");
  item.setAttribute("aria-labelledby", id);
  const heading = document.createElement("div");
  heading.className = "group";
  heading.id = id;
  heading.textContent = label;
  const inner = document.createElement("ul");
  inner.setAttribute("role", "none");
  inner.append(...results.map(option));
  item.append(heading, inner);
  list.append(item);
}

function options(): HTMLLIElement[] {
  return [...(list?.querySelectorAll<HTMLLIElement>('[role="option"]') ?? [])];
}

function highlight(index: number, scroll = true): void {
  const all = options();
  if (all.length === 0) {
    input?.removeAttribute("aria-activedescendant");
    return;
  }
  active = (index + all.length) % all.length;
  all.forEach((element, i) => element.setAttribute("aria-selected", String(i === active)));
  const current = all[active];
  if (!current) return;
  input?.setAttribute("aria-activedescendant", current.id);
  if (scroll) current.scrollIntoView({ block: "nearest" });
}

async function render(query: string): Promise<void> {
  if (!list || !empty) return;
  const run = ++generation;
  list.replaceChildren();
  optionCount = 0;
  groupCount = 0;

  const matches = rank(entries, query);
  const sections = [...new Set(matches.map((entry) => entry.section))];
  for (const section of sections) {
    group(
      section,
      matches
        .filter((entry) => entry.section === section)
        .map((entry) => ({ title: entry.title, detail: entry.description, href: entry.href })),
    );
  }

  const trimmed = query.trim();
  if (trimmed.length >= 3) {
    const engine = await loadPagefind();
    if (engine && run === generation) {
      const found = await engine.search(trimmed);
      const data = await Promise.all(found.results.slice(0, 6).map((result) => result.data()));
      if (run !== generation) return;
      const known = new Set(matches.map((entry) => entry.href));
      group(
        "In the docs",
        data
          .filter((result) => !known.has(result.url))
          .map((result) => ({ title: result.meta.title ?? result.url, detail: result.excerpt, href: result.url, html: true })),
      );
    }
  }

  const count = options().length;
  empty.hidden = count > 0;
  input?.setAttribute("aria-expanded", String(count > 0));
  if (status) status.textContent = count === 0 ? "No results" : `${count} ${count === 1 ? "result" : "results"}`;
  highlight(0);
}

function open(): void {
  if (!dialog || !input || dialog.open) return;
  returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  dialog.showModal();
  input.value = "";
  void render("");
  input.focus();
}

function close(): void {
  dialog?.close();
}

dialog?.addEventListener("close", () => {
  if (returnFocus?.isConnected && !(returnFocus instanceof HTMLIFrameElement)) returnFocus.focus();
  returnFocus = null;
});

document.addEventListener("keydown", (event) => {
  const typing = event.target instanceof HTMLElement && event.target.closest("input, textarea, [contenteditable]");
  if ((event.key === "k" && (event.metaKey || event.ctrlKey)) || (event.key === "/" && !typing)) {
    event.preventDefault();
    open();
  }
});

for (const trigger of document.querySelectorAll("[data-search-open]")) {
  trigger.addEventListener("click", open);
}

for (const button of document.querySelectorAll("[data-search-close]")) {
  button.addEventListener("click", close);
}

input?.addEventListener("input", () => void render(input.value));

input?.addEventListener("keydown", (event) => {
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      highlight(active + 1);
      break;
    case "ArrowUp":
      event.preventDefault();
      highlight(active - 1);
      break;
    case "Enter":
      event.preventDefault();
      options()[active]?.querySelector("a")?.click();
      break;
    case "Escape":
      event.preventDefault();
      close();
      break;
  }
});

dialog?.addEventListener("click", (event) => {
  if (event.target === dialog) close();
});
