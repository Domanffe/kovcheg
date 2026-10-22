import { buildSource } from "./build";
import { SOURCE_IDS } from "./meta";
import type { Source, SourceId } from "./types";

const LOADERS: Record<SourceId, () => Promise<Source>> = {
  fitgirl: () => import("./fitgirl").then((m) => buildSource("fitgirl", m.search)),
  "online-fix": () => import("./online-fix").then((m) => buildSource("online-fix", m.search)),
  xatab: () => import("./xatab").then((m) => buildSource("xatab", m.search)),
  "tpb-games": () => import("./piratebay").then((m) => buildSource("tpb-games", m.searchGames)),
  yts: () => import("./yts").then((m) => buildSource("yts", m.search)),
  "tpb-movies": () => import("./piratebay").then((m) => buildSource("tpb-movies", m.searchMovies)),
  "x1337-movies": () => import("./x1337").then((m) => buildSource("x1337-movies", m.searchMovies)),
  "bittorrented-movies": () =>
    import("./bittorrented").then((m) => buildSource("bittorrented-movies", m.searchMovies)),
  "tgx-movies": () => import("./torrentgalaxy").then((m) => buildSource("tgx-movies", m.search)),
  eztv: () => import("./eztv").then((m) => buildSource("eztv", m.search)),
  "tpb-tv": () => import("./piratebay").then((m) => buildSource("tpb-tv", m.searchTv)),
  "x1337-tv": () => import("./x1337").then((m) => buildSource("x1337-tv", m.searchTv)),
  "bittorrented-tv": () =>
    import("./bittorrented").then((m) => buildSource("bittorrented-tv", m.searchTv)),
  torlock: () => import("./torlock").then((m) => buildSource("torlock", m.search)),
  nyaa: () => import("./nyaa").then((m) => buildSource("nyaa", m.search)),
  subsplease: () => import("./subsplease").then((m) => buildSource("subsplease", m.search)),
  anilibria: () => import("./anilibria").then((m) => buildSource("anilibria", m.search)),
};

const loaded = new Map<SourceId, Source>();

async function loadSource(id: SourceId): Promise<Source> {
  const hit = loaded.get(id);
  if (hit) return hit;
  const source = await LOADERS[id]();
  loaded.set(id, source);
  return source;
}

export async function loadAllSources(): Promise<Source[]> {
  return Promise.all(SOURCE_IDS.map(loadSource));
}
