import type { SourceId } from "./types";

const COLOR = {
  accent: "#a78bfa",
  alt: "#b9a7e6",
  good: "#50e3c2",
  warn: "#f5a623",
  bright: "#fafafa",
} as const;

export const SOURCE_STYLE: Record<SourceId, { tag: string; color: string }> = {
  fitgirl: { tag: "FG", color: COLOR.accent },
  "online-fix": { tag: "OF", color: "#7dd3fc" },
  xatab: { tag: "XAT", color: "#f97316" },
  yts: { tag: "YTS", color: COLOR.good },
  eztv: { tag: "EZTV", color: COLOR.warn },
  nyaa: { tag: "NYAA", color: COLOR.bright },
  subsplease: { tag: "SUB", color: "#b9a7e6" },
  anilibria: { tag: "AL", color: "#fb7185" },
  "tpb-movies": { tag: "TPB", color: "#14B8A6" },
  "tpb-tv": { tag: "TPB", color: "#14B8A6" },
  "tpb-games": { tag: "TPB", color: "#14B8A6" },
  "x1337-movies": { tag: "1337", color: "#f6a55c" },
  "x1337-tv": { tag: "1337", color: "#f6a55c" },
  "bittorrented-movies": { tag: "BTD", color: "#84cc16" },
  "bittorrented-tv": { tag: "BTD", color: "#84cc16" },
  "tgx-movies": { tag: "TGx", color: "#38bdf8" },
  torlock: { tag: "TLK", color: "#a3e635" },
};

// Tolerant lookup: a source id may be absent (a pasted magnet / bare infohash) or
// no longer exist (a removed source persisted in old history/seeds). Fall back to a
// neutral tag rather than indexing SOURCE_STYLE and crashing on `undefined`.
export function sourceStyle(id?: string): { tag: string; color: string } {
  const s = id ? (SOURCE_STYLE as Record<string, { tag: string; color: string }>)[id] : undefined;
  return s ?? { tag: "•", color: COLOR.alt };
}
