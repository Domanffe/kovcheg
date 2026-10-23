import { decodeResponseText, fetchResilient, HttpError, USER_AGENT } from "../util/net";
import { dedupeByUrl } from "./build";
import { buildMagnet } from "./magnet";
import { unescapeEntities } from "./rss";
import { magnetFromTorrentBytes } from "./torrentBytes";
import type { SearchOptions, TorrentResult } from "./types";

const HOME = "https://online-fix.me";
const RSS_URL = `${HOME}/rss.xml`;
const HEADERS = { "User-Agent": USER_AGENT, Referer: `${HOME}/` };
const MAX_RESULTS = 10;
const MAX_CONCURRENCY = 3;

export interface GameLink {
  title: string;
  url: string;
  added?: number;
}

function searchUrl(query: string): string {
  return `${HOME}/index.php?do=search&subaction=search&story=${encodeURIComponent(query.trim())}`;
}

export function parseDleSearchResults(html: string): GameLink[] {
  const links: GameLink[] = [];
  const re =
    /<a href="(https:\/\/online-fix\.me\/games\/[^"]+\.html)"[^>]*>\s*<h2 class="title">\s*([^<]+)/gi;
  for (const match of html.matchAll(re)) {
    links.push({ title: unescapeEntities(match[2]!.trim()), url: match[1]! });
  }
  return dedupeByUrl(links, MAX_RESULTS);
}

export function parseDleRssGames(xml: string): GameLink[] {
  const links: GameLink[] = [];
  for (const item of xml.split("<item>").slice(1)) {
    const url = item.match(/<link>(.*?)<\/link>/)?.[1]?.trim();
    if (!url?.includes("/games/")) continue;
    const title = unescapeEntities(item.match(/<title>(.*?)<\/title>/)?.[1]?.trim() ?? "Unknown");
    const addedStr = item.match(/<pubDate>(.*?)<\/pubDate>/)?.[1] ?? "";
    const added = addedStr ? new Date(addedStr).getTime() / 1000 : undefined;
    links.push({ title, url, added });
  }
  return dedupeByUrl(links, MAX_RESULTS);
}

export function parseTorrentDirFromPage(html: string): string | null {
  const match = html.match(
    /href="(https:\/\/uploads\.online-fix\.me:\d+\/torrents\/[^"]+)"/i,
  );
  return match?.[1] ?? null;
}

export function parseTorrentFileFromListing(html: string, baseUrl: string): string | null {
  const match = html.match(/href="([^"?]+\.torrent)"/i);
  if (!match) return null;
  const href = match[1]!;
  if (/^https?:\/\//i.test(href)) return href;
  return new URL(href, baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`).href;
}

async function fetchText(url: string, opts: SearchOptions): Promise<string> {
  const res = await fetchResilient(url, { headers: HEADERS, signal: opts.signal });
  if (!res.ok) throw new HttpError(res.status, `online-fix returned ${res.status}`);
  return decodeResponseText(res);
}

async function resolveMagnet(
  link: GameLink,
  opts: SearchOptions,
): Promise<TorrentResult | null> {
  try {
    const page = await fetchText(link.url, opts);
    const torrentDir = parseTorrentDirFromPage(page);
    if (!torrentDir) return null;

    const listing = await fetchText(torrentDir, opts);
    const torrentUrl = parseTorrentFileFromListing(listing, torrentDir);
    if (!torrentUrl) return null;

    const res = await fetchResilient(torrentUrl, { headers: HEADERS, signal: opts.signal });
    if (!res.ok) return null;

    const parsed = await magnetFromTorrentBytes(new Uint8Array(await res.arrayBuffer()));
    if (!parsed) return null;

    return {
      infoHash: parsed.infoHash,
      name: link.title,
      sizeBytes: parsed.sizeBytes,
      seeders: 0,
      leechers: 0,
      source: "online-fix",
      magnet: buildMagnet(parsed.infoHash, link.title),
      added: link.added,
    };
  } catch {
    return null;
  }
}

async function resolveAll(links: GameLink[], opts: SearchOptions): Promise<TorrentResult[]> {
  const out: TorrentResult[] = [];
  for (let i = 0; i < links.length; i += MAX_CONCURRENCY) {
    const batch = links.slice(i, i + MAX_CONCURRENCY);
    const hits = await Promise.all(batch.map((link) => resolveMagnet(link, opts)));
    for (const hit of hits) {
      if (hit) out.push(hit);
    }
  }
  return out;
}

export async function search(query: string, opts: SearchOptions = {}): Promise<TorrentResult[]> {
  const q = query.trim();
  const listUrl = q ? searchUrl(q) : RSS_URL;
  const html = await fetchText(listUrl, opts);
  const links = q ? parseDleSearchResults(html) : parseDleRssGames(html);
  if (links.length === 0) return [];
  return resolveAll(links, opts);
}
