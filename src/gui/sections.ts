import type { Section } from "./App";

export const SECTION_ORDER: Section[] = [
  "all",
  "games",
  "movies",
  "tv",
  "anime",
  "library",
  "downloads",
  "seeding",
];

export const SEARCH_SECTIONS = new Set<Section>(["all", "games", "movies", "tv", "anime"]);

export const CATEGORY_GROUP: Partial<Record<Section, string>> = {
  games: "Games",
  movies: "Movies",
  tv: "TV",
  anime: "Anime",
};
