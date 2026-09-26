export const SITE = "https://sam-ruff.github.io/iced-cube";

/** Removes YAML frontmatter from a Markdown source. */
export function stripFrontmatter(source: string): string {
  return source.replace(/^---\n[\s\S]*?\n---\n/, "").trim();
}

/**
 * Turns site-relative links, such as `../components/tabs/` on the page at
 * `from`, into absolute links to the Markdown version of the target page.
 */
export function absoluteLinks(markdown: string, from: string): string {
  return markdown.replace(/\]\((?!https?:|#|mailto:)([^)]+)\)/g, (_, target: string) => {
    const url = new URL(target, `${SITE}/${from}`);
    const path = url.pathname.replace(/\/$/, "");
    // Files such as llms.txt keep their name; only pages gain `.md`.
    if (/\.[a-z0-9]+$/i.test(path)) return `](${url.origin}${path}${url.hash})`;
    const page = path.endsWith("/iced-cube/docs") ? `${path}/index` : path;
    return `](${url.origin}${page}.md${url.hash})`;
  });
}
