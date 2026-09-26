import { getCollection, type CollectionEntry } from "astro:content";
import { GROUPS } from "../content.config";
import { href } from "./paths";

export type Component = CollectionEntry<"components">;

export interface NavLink {
  title: string;
  href: string;
  description: string;
  keywords?: string[];
}

export interface NavSection {
  title: string;
  links: NavLink[];
}

export const guides: NavLink[] = [
  { title: "Introduction", href: href("/docs/"), description: "What iced-cube is and how it fits with iced." },
  { title: "Installation", href: href("/docs/installation/"), description: "Add the crate and render your first component." },
  { title: "Theming", href: href("/docs/theming/"), description: "Light and dark themes, tokens and custom palettes." },
  { title: "Icons", href: href("/docs/icons/"), description: "Lucide icons resolved at compile time." },
  { title: "Subscriptions", href: href("/docs/subscriptions/"), description: "Feeding components from background work with channels." },
  { title: "Keyboard shortcuts", href: href("/docs/keyboard/"), description: "Default shortcuts and how to change them with a keymap." },
  { title: "Testing", href: href("/docs/testing/"), description: "Headless tests for your own UI with iced_test." },
  { title: "Status", href: href("/docs/status/"), description: "Version, platforms, minimum Rust and what is planned." },
];

export const allComponents: NavLink = {
  title: "All components",
  href: href("/docs/components/"),
  description: "Every component, grouped, with previews.",
};

/** Components in reading order: by group, then by `order` within the group. */
export async function components(): Promise<Component[]> {
  const entries = await getCollection("components");
  const group = (entry: Component) => GROUPS.indexOf(entry.data.group);
  return entries.sort((a, b) => group(a) - group(b) || a.data.order - b.data.order);
}

export function componentHref(entry: Component): string {
  return href(`/docs/components/${entry.id}/`);
}

export async function sections(): Promise<NavSection[]> {
  const all = await components();
  const grouped = GROUPS.map((group) => ({
    title: group,
    links: all
      .filter((entry) => entry.data.group === group)
      .map((entry) => ({
        title: entry.data.title,
        href: componentHref(entry),
        description: entry.data.description,
        keywords: entry.data.keywords,
      })),
  })).filter((section) => section.links.length > 0);

  return [{ title: "Getting started", links: guides }, ...grouped];
}
