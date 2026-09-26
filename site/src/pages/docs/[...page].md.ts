import type { APIRoute, GetStaticPaths } from "astro";
import { markdownPages, renderPage, type MarkdownPage } from "../../lib/markdown";

// Serves docs/<guide>.md and docs/components/<slug>.md.
export const getStaticPaths: GetStaticPaths = async () =>
  (await markdownPages()).map((page) => ({
    params: { page: page.path.replace(/^docs\//, "").replace(/\.md$/, "") },
    props: { page },
  }));

export const GET: APIRoute = ({ props }) =>
  new Response(renderPage((props as { page: MarkdownPage }).page), {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
