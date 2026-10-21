import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";

export const GROUPS = ["Actions", "Forms", "Layout", "Navigation", "Overlays", "Data", "Feedback", "Application"] as const;

const components = defineCollection({
  loader: glob({ pattern: "*.md", base: "./src/content/components" }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    group: z.enum(GROUPS),
    order: z.number(),
    module: z.string(),
    /** The `use` lines shown under the title. Falls back to the module path. */
    imports: z.string().optional(),
    /** Other words people search for, such as "toggle" for a switch. */
    keywords: z.array(z.string()).default([]),
    /** Slugs of closely related component pages. */
    related: z.array(z.string()).default([]),
    hero: z.string(),
    stories: z.array(z.string()).min(1),
    api: z.array(z.object({ name: z.string(), description: z.string() })).default([]),
    keyboard: z.array(z.object({ keys: z.string(), action: z.string() })).default([]),
  }),
});

export const collections = { components };
