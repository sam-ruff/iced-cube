import { frameHeight } from "../lib/frame";
import { plan } from "../lib/pool";
import { currentTheme, type Theme } from "./theme";

const CAPACITY = 8;
/** How long the wasm download may take before giving up. */
const FETCH_TIMEOUT_MS = 120_000;
/** How long iced may take to draw once the wasm has arrived. */
const BOOT_TIMEOUT_MS = 20_000;
const mock = import.meta.env.PUBLIC_PREVIEW_MODE === "mock";
const previewUrl = `${import.meta.env.BASE_URL.replace(/\/$/, "")}/preview/index.html`;
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

type Frame = HTMLElement;
type PreviewMessage = { type?: string; back?: boolean; height?: unknown };

const frames = [...document.querySelectorAll<Frame>("[data-story]")].filter((frame) => !frame.dataset["bound"]);
const wanted = new Set<Frame>();
const mounted = new Map<Frame, { iframe: HTMLIFrameElement; timer: number }>();
/** Frames the reader asked to run while reduced motion is on. */
const started = new Set<Frame>();

function gpuAvailable(): boolean {
  if ("gpu" in navigator) return true;
  try {
    return document.createElement("canvas").getContext("webgl2") !== null;
  } catch {
    return false;
  }
}

const live = !mock && gpuAvailable();

function setStatus(frame: Frame, state: "static" | "loading" | "live" | "failed", text: string): void {
  frame.classList.toggle("loading", state === "loading");
  frame.classList.toggle("live", state === "live");
  frame.classList.toggle("failed", state === "failed");
  for (const poster of frame.querySelectorAll("img")) {
    if (state === "live") poster.setAttribute("aria-hidden", "true");
    else poster.removeAttribute("aria-hidden");
  }
  const label = frame.querySelector<HTMLElement>("[data-status-text]");
  if (label) label.textContent = text;
}

function distance(frame: Frame): number {
  const rect = frame.getBoundingClientRect();
  if (rect.bottom < 0) return -rect.bottom;
  if (rect.top > innerHeight) return rect.top - innerHeight;
  return 0;
}

function title(frame: Frame): string {
  return frame.dataset["title"] ?? frame.dataset["story"] ?? "Preview";
}

function fail(frame: Frame): void {
  unmount(frame);
  setStatus(frame, "failed", "Live preview failed to load");
}

function mount(frame: Frame): void {
  const story = frame.dataset["story"];
  if (!story || mounted.has(frame)) return;

  const iframe = document.createElement("iframe");
  iframe.title = `Live preview: ${title(frame)}`;
  const wheel = frame.dataset["wheel"] === "true" ? "&wheel=1" : "";
  iframe.src = `${previewUrl}?story=${encodeURIComponent(story)}&theme=${currentTheme()}${wheel}`;
  iframe.loading = "eager";
  const timer = window.setTimeout(() => fail(frame), FETCH_TIMEOUT_MS);

  mounted.set(frame, { iframe, timer });
  setStatus(frame, "loading", "Loading live preview");
  frame.append(iframe);
}

function unmount(frame: Frame): void {
  const entry = mounted.get(frame);
  if (!entry) return;
  window.clearTimeout(entry.timer);
  entry.iframe.remove();
  mounted.delete(frame);
  setStatus(frame, "static", "Static preview");
}

function allowed(frame: Frame): boolean {
  return !reducedMotion.matches || started.has(frame);
}

function schedule(): void {
  if (!live) return;
  const next = plan([...wanted].filter(allowed), [...mounted.keys()], CAPACITY, distance);
  next.unmount.forEach(unmount);
  next.mount.forEach(mount);
}

function frameOf(source: MessageEventSource | null): [Frame, { iframe: HTMLIFrameElement; timer: number }] | undefined {
  for (const entry of mounted) {
    if (entry[1].iframe.contentWindow === source) return entry;
  }
  return undefined;
}

/** Moves focus to the next or previous focusable element outside a preview. */
function leave(iframe: HTMLIFrameElement, back: boolean): void {
  const focusable = [
    ...document.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), input:not([disabled]), iframe, [tabindex]:not([tabindex="-1"])',
    ),
  ].filter((element) => element.offsetParent !== null || element === iframe);
  const index = focusable.indexOf(iframe);
  const target = focusable[back ? index - 1 : index + 1];
  if (target) target.focus();
  else iframe.blur();
}

window.addEventListener("message", (event: MessageEvent<PreviewMessage>) => {
  const found = frameOf(event.source);
  if (!found) return;
  const [frame, entry] = found;

  switch (event.data?.type) {
    case "fetched":
      window.clearTimeout(entry.timer);
      entry.timer = window.setTimeout(() => fail(frame), BOOT_TIMEOUT_MS);
      break;
    case "ready":
      window.clearTimeout(entry.timer);
      setStatus(frame, "live", "Live");
      break;
    case "error":
      fail(frame);
      break;
    case "exit":
      leave(entry.iframe, event.data.back === true);
      break;
    case "size": {
      // Stories whose rows wrap on a narrow screen report the height they need.
      const base = Number(frame.dataset["height"]);
      if (Number.isFinite(base)) frame.style.height = `${frameHeight(base, event.data.height)}px`;
      break;
    }
    case "search":
      document.querySelector<HTMLElement>("[data-search-open]")?.click();
      break;
  }
});

function updateStandaloneTheme(theme: Theme): void {
  for (const link of document.querySelectorAll<HTMLAnchorElement>("[data-standalone-preview]")) {
    const url = new URL(link.href);
    url.searchParams.set("theme", theme);
    link.href = url.href;
  }
}

updateStandaloneTheme(currentTheme());

window.addEventListener("themechange", (event) => {
  const theme = (event as CustomEvent<Theme>).detail;
  updateStandaloneTheme(theme);
  for (const { iframe } of mounted.values()) {
    iframe.contentWindow?.postMessage({ type: "theme", value: theme }, "*");
  }
});

const near = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      const frame = entry.target as Frame;
      if (entry.isIntersecting) wanted.add(frame);
      else wanted.delete(frame);
    }
    schedule();
  },
  { rootMargin: "300px 0px" },
);

const far = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) unmount(entry.target as Frame);
    }
    schedule();
  },
  { rootMargin: "1200px 0px" },
);

function addStartButton(frame: Frame): void {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "start";
  button.textContent = "Start live preview";
  button.setAttribute("aria-label", `Start live preview: ${title(frame)}`);
  button.addEventListener("click", () => {
    started.add(frame);
    button.remove();
    schedule();
  });
  frame.append(button);
}

for (const frame of frames) {
  frame.dataset["bound"] = "true";
  if (mock) setStatus(frame, "static", "Mock preview");
  else if (!live) setStatus(frame, "static", "Static image: this browser has no WebGL2 or WebGPU");
  else if (reducedMotion.matches) addStartButton(frame);
  near.observe(frame);
  far.observe(frame);
}

for (const root of document.querySelectorAll<HTMLElement>("[data-preview-root]")) {
  if (root.dataset["bound"]) continue;
  root.dataset["bound"] = "true";

  const tabs = [...root.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  const copy = root.querySelector<HTMLButtonElement>("[data-copy]");
  const select = (tab: HTMLButtonElement): void => {
    if (copy) copy.hidden = tab.dataset["tab"] !== "code";
    for (const other of tabs) {
      const selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
      const panel = root.querySelector<HTMLElement>(`[data-panel="${other.dataset["tab"]}"]`);
      if (panel) panel.hidden = !selected;
    }
  };
  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => select(tab));
    tab.addEventListener("keydown", (event) => {
      const targets: Record<string, number> = {
        ArrowRight: (index + 1) % tabs.length,
        ArrowLeft: (index - 1 + tabs.length) % tabs.length,
        Home: 0,
        End: tabs.length - 1,
      };
      const next = tabs[targets[event.key] ?? -1];
      if (!next) return;
      event.preventDefault();
      select(next);
      next.focus();
    });
  });

  root.querySelector("[data-reset]")?.addEventListener("click", () => {
    const frame = root.querySelector<Frame>("[data-story]");
    if (!frame || !live) return;
    started.add(frame);
    frame.querySelector(".start")?.remove();
    unmount(frame);
    wanted.add(frame);
    schedule();
  });

  const status = root.querySelector<HTMLElement>("[data-copy-status]");
  copy?.addEventListener("click", async () => {
    const pre = root.querySelector("pre");
    const label = copy.querySelector(".copy-label");
    try {
      await navigator.clipboard.writeText(pre?.textContent ?? "");
      copy.classList.add("copied");
      if (label) label.textContent = "Copied";
      if (status) status.textContent = "Code copied";
    } catch {
      if (pre) window.getSelection()?.selectAllChildren(pre);
      const hint = /Mac|iPhone|iPad/.test(navigator.platform) ? "Press ⌘C" : "Press Ctrl+C";
      if (label) label.textContent = hint;
      if (status) status.textContent = `Code selected. ${hint} to copy.`;
    }
    window.setTimeout(() => {
      copy.classList.remove("copied");
      if (label) label.textContent = "Copy";
      if (status) status.textContent = "";
    }, 1600);
  });
}
