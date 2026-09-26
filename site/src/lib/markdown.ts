// Builds the plain Markdown version of the docs for AI agents and other tools.
// Everything comes from the same sources as the HTML pages, so the two cannot drift.
import { guides, components, type Component } from "./nav";
import { keymapFor } from "./keymaps";
import { absoluteLinks, SITE, stripFrontmatter } from "./markdown-text";
import { story } from "./stories";

// Raw guide sources, keyed by file path.
const guideSources = import.meta.glob<string>("../pages/docs/*.{md,mdx}", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** Drops MDX imports and replaces each live preview with its example code. */
function plainMdx(source: string): string {
  return source
    .replace(/^import .+;\n/gm, "")
    .replace(/<Preview\s+id="([^"]+)"[^>]*\/>/g, (_, id: string) => {
      const example = story(id);
      return `${example.title}: ${example.description}\n\n\`\`\`rust\n${example.source.trim()}\n\`\`\``;
    });
}

export interface MarkdownPage {
  /** Path relative to the site root, such as `docs/components/button.md`. */
  path: string;
  title: string;
  description: string;
  body: string;
}

function guidePages(): MarkdownPage[] {
  return guides.flatMap((guide) => {
    const slug = guide.href.replace(/^.*\/docs\//, "").replace(/\/$/, "") || "index";
    const source = guideSources[`../pages/docs/${slug}.md`] ?? guideSources[`../pages/docs/${slug}.mdx`];
    if (!source) return [];
    const from = slug === "index" ? "docs/" : `docs/${slug}/`;
    return [
      {
        path: `docs/${slug}.md`,
        title: guide.title,
        description: guide.description,
        body: absoluteLinks(plainMdx(stripFrontmatter(source)), from),
      },
    ];
  });
}

function componentBody(entry: Component): string {
  const { data } = entry;
  const from = `docs/components/${entry.id}/`;
  const parts: string[] = [];

  parts.push("```rust\n" + (data.imports ?? `use iced_cube::${data.module};`).trim() + "\n```");
  if (entry.body) parts.push("## Usage\n\n" + absoluteLinks(entry.body.trim(), from));

  parts.push("## Examples");
  for (const id of data.stories) {
    const example = story(id);
    parts.push(`### ${example.title}\n\n${example.description}\n\n\`\`\`rust\n${example.source.trim()}\n\`\`\``);
  }

  if (data.api.length > 0) {
    parts.push("## API\n\n" + data.api.map((item) => `- \`${item.name}\`: ${item.description}`).join("\n"));
  }

  const bindings = keymapFor(entry.id);
  const builtIn = data.keyboard;
  if (bindings.length > 0 || builtIn.length > 0) {
    const lines = [
      ...bindings.map((binding) => `- ${binding.keys.join(" or ")}: \`${binding.action}\`. ${binding.description}`),
      ...builtIn.map((item) => `- ${item.keys}: ${item.action}`),
    ];
    const note =
      bindings.length > 0
        ? `\n\nThese are defaults; change them with a keymap (${SITE}/docs/keyboard.md).`
        : "";
    parts.push("## Keyboard\n\n" + lines.join("\n") + note);
  }

  if (data.related.length > 0) {
    parts.push("## See also\n\n" + data.related.map((slug) => `- ${SITE}/docs/components/${slug}.md`).join("\n"));
  }

  return parts.join("\n\n");
}

async function componentPages(): Promise<MarkdownPage[]> {
  return (await components()).map((entry) => ({
    path: `docs/components/${entry.id}.md`,
    title: entry.data.title,
    description: entry.data.description,
    body: componentBody(entry),
  }));
}

/** Every docs page as plain Markdown, guides first, then components in reading order. */
export async function markdownPages(): Promise<MarkdownPage[]> {
  return [...guidePages(), ...(await componentPages())];
}

export function renderPage(page: MarkdownPage): string {
  return `# ${page.title}\n\n> ${page.description}\n\n${page.body}\n`;
}

/** The llms.txt index: a title, a summary and a link to every page. */
export async function llmsIndex(): Promise<string> {
  const pages = await markdownPages();
  const list = (items: MarkdownPage[]) =>
    items.map((page) => `- [${page.title}](${SITE}/${page.path}): ${page.description}`).join("\n");
  const guideList = pages.filter((page) => !page.path.startsWith("docs/components/"));
  const componentList = pages.filter((page) => page.path.startsWith("docs/components/"));

  return [
    "# iced-cube",
    "",
    "> Themeable application components for iced 0.14, the Rust GUI library, with Lucide icons. Every page is also available as plain Markdown, and the whole set is in one file at " +
      `${SITE}/llms-full.txt.`,
    "",
    "## Guides",
    "",
    list(guideList),
    "",
    "## Components",
    "",
    list(componentList),
    "",
  ].join("\n");
}

/** Every page in one file. */
export async function llmsFull(): Promise<string> {
  const pages = await markdownPages();
  return pages.map(renderPage).join("\n---\n\n");
}
