import { z } from "astro/zod";

/** One default shortcut of a component, exported from the Rust keymaps. */
export const Binding = z.object({
  /** Alternative key chords for the same action, such as ["Right", "Down"]. */
  keys: z.array(z.string()).min(1),
  action: z.string(),
  description: z.string(),
});

export type Binding = z.infer<typeof Binding>;

const Keymaps = z.record(z.string(), z.array(Binding));

// Written by scripts/prepare.mjs; absent until a component has a keymap.
const files = import.meta.glob<{ default: unknown }>("../generated/keymaps.json", { eager: true });
const raw = Object.values(files)[0]?.default ?? {};
const keymaps = Keymaps.parse(raw);

/** Default bindings for a docs page slug, empty when it has none. */
export function keymapFor(slug: string): Binding[] {
  return keymaps[slug] ?? [];
}
