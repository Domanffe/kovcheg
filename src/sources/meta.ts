import type { SourceGroup, SourceId } from "./types";

export interface SourceMeta {
  id: SourceId;
  label: string;
  group: SourceGroup;
  homepage: string;
  reportsHealth: boolean;
  browseOnly?: boolean;
  /** Skipped by the search: answers 401/403/5xx from the RU network as of 2026-10-23.
   *  Re-check before trusting this list — the same sites often work from other networks. */
  disabled?: boolean;
}

export const SOURCE_IDS: readonly SourceId[] = [
  "fitgirl",
  "online-fix",
  "xatab",
  "tpb-games",
  "yts",
  "tpb-movies",
  "x1337-movies",
  "bittorrented-movies",
  "tgx-movies",
  "eztv",
  "tpb-tv",
  "x1337-tv",
  "bittorrented-tv",
  "torlock",
  "nyaa",
  "subsplease",
  "anilibria",
] as const;

export const SOURCE_META: Record<SourceId, SourceMeta> = {
  fitgirl: {
    id: "fitgirl",
    label: "FitGirl",
    group: "Games",
    homepage: "https://fitgirl-repacks.site",
    reportsHealth: false,
  },
  "online-fix": {
    id: "online-fix",
    label: "Online-Fix",
    group: "Games",
    homepage: "https://online-fix.me",
    reportsHealth: false,
  },
  xatab: {
    id: "xatab",
    label: "Xatab",
    group: "Games",
    homepage: "https://byxatab.com",
    reportsHealth: false,
  },
  "tpb-games": {
    id: "tpb-games",
    label: "TPB",
    group: "Games",
    homepage: "https://thepiratebay.org",
    reportsHealth: true,
  },
  yts: {
    id: "yts",
    label: "YTS",
    group: "Movies",
    homepage: "https://yts.mx",
    reportsHealth: true,
  },
  "tpb-movies": {
    id: "tpb-movies",
    label: "TPB",
    group: "Movies",
    homepage: "https://thepiratebay.org",
    reportsHealth: true,
  },
  "x1337-movies": {
    id: "x1337-movies",
    label: "1337x",
    group: "Movies",
    homepage: "https://1337x.to",
    reportsHealth: true,
    disabled: true,
  },
  "bittorrented-movies": {
    id: "bittorrented-movies",
    label: "BitTorrented",
    group: "Movies",
    homepage: "https://bittorrented.com",
    reportsHealth: true,
    disabled: true,
  },
  "tgx-movies": {
    id: "tgx-movies",
    label: "TGx",
    group: "Movies",
    homepage: "https://torrentgalaxy.to",
    reportsHealth: true,
    disabled: true,
  },
  eztv: {
    id: "eztv",
    label: "EZTV",
    group: "TV",
    homepage: "https://eztvx.to",
    reportsHealth: true,
  },
  "tpb-tv": {
    id: "tpb-tv",
    label: "TPB",
    group: "TV",
    homepage: "https://thepiratebay.org",
    reportsHealth: true,
  },
  "x1337-tv": {
    id: "x1337-tv",
    label: "1337x",
    group: "TV",
    homepage: "https://1337x.to",
    reportsHealth: true,
    disabled: true,
  },
  "bittorrented-tv": {
    id: "bittorrented-tv",
    label: "BitTorrented",
    group: "TV",
    homepage: "https://bittorrented.com",
    reportsHealth: true,
    disabled: true,
  },
  torlock: {
    id: "torlock",
    label: "Torlock",
    group: "TV",
    homepage: "https://www.torlock.com",
    reportsHealth: true,
    disabled: true,
  },
  nyaa: {
    id: "nyaa",
    label: "Nyaa",
    group: "Anime",
    homepage: "https://nyaa.si",
    reportsHealth: true,
  },
  subsplease: {
    id: "subsplease",
    label: "SubsPlease",
    group: "Anime",
    homepage: "https://subsplease.org",
    reportsHealth: false,
  },
  anilibria: {
    id: "anilibria",
    label: "AniLibria",
    group: "Anime",
    homepage: "https://anilibria.top",
    reportsHealth: true,
  },
};

export function getSourceMeta(id: SourceId): SourceMeta {
  return SOURCE_META[id];
}

export function allSourceMeta(): SourceMeta[] {
  return ACTIVE_SOURCE_IDS.map((id) => SOURCE_META[id]);
}

/** Sources that still answer. A disabled source keeps its meta so old history keeps its tag. */
export const ACTIVE_SOURCE_IDS: readonly SourceId[] = SOURCE_IDS.filter(
  (id) => !SOURCE_META[id].disabled,
);
