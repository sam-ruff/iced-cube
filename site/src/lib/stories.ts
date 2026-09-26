import { z } from "astro/zod";
import raw from "../generated/stories.json";

export const Story = z.object({
  id: z.string(),
  component: z.string(),
  title: z.string(),
  description: z.string(),
  file: z.string(),
  source: z.string(),
  /** Preview frame height in pixels, matching the story's snapshot. */
  height: z.number().default(280),
});

export type Story = z.infer<typeof Story>;

export const stories: Story[] = z.array(Story).parse(raw);

const byId = new Map(stories.map((story) => [story.id, story]));

/** Looks up a story, failing the build when a docs page references one that does not exist. */
export function story(id: string): Story {
  const found = byId.get(id);
  if (!found) {
    throw new Error(`Unknown story "${id}". Known: ${[...byId.keys()].join(", ")}`);
  }
  return found;
}

/** File name of a story's poster image under public/snapshots. */
export function posterName(id: string, theme: "light" | "dark"): string {
  return `${id.replaceAll("/", "--")}-${theme}.png`;
}
