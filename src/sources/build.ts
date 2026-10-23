import { SOURCE_META } from "./meta";
import type { Source, SourceId } from "./types";

export function buildSource(id: SourceId, search: Source["search"]): Source {
  return { ...SOURCE_META[id], search };
}

/** Keeps the first link per URL, up to `limit`, so a page full of repeats cannot flood results. */
export function dedupeByUrl<T extends { url: string }>(links: T[], limit: number): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const link of links) {
    if (seen.has(link.url)) continue;
    seen.add(link.url);
    out.push(link);
    if (out.length >= limit) break;
  }
  return out;
}
