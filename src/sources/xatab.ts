import { decodeResponseText, fetchResilient, HttpError, USER_AGENT } from "../util/net";
import { dedupeByUrl } from "./build";
import { buildMagnet } from "./magnet";
import { unescapeEntities } from "./rss";
import { magnetFromTorrentBytes } from "./torrentBytes";
import type { SearchOptions, TorrentResult } from "./types";

const HOME = "https://byxatab.com";
const HEADERS = { "User-Agent": USER_AGENT, Referer: `${HOME}/` };
const MAX_RESULTS = 10;
const MAX_CONCURRENCY = 3;

export interface GameLink {
  title: string;
  url: string;
  added?: number;
}

function searchBody(query: string): string {
  return new URLSearchParams({
    do: "search",
    subaction: "search",
    story: query.trim(),
  }).toString();
}

export function parseXatabSearchResults(html: string): GameLink[] {
  const links: GameLink[] = [];
  const gridRe =
    /<a href="(https:\/\/byxatab\.com\/games\/[^"]+)"[^>]*class="item grid-item[^"]*"[\s\S]*?<div class="item__title">([^<]+)/gi;
  for (const match of html.matchAll(gridRe)) {
    links.push({ title: unescapeEntities(match[2]!.trim()), url: match[1]! });
  }

  if (links.length === 0) {
    const legacyRe = /<a href="(https:\/\/byxatab\.com\/[^"]+\.html)"[^>]*>\s*([^<]+)/gi;
    for (const match of html.matchAll(legacyRe)) {
      links.push({ title: unescapeEntities(match[2]!.trim()), url: match[1]! });
    }
  }

  return dedupeByUrl(links, MAX_RESULTS);
}

export function parseMagnetFromPage(html: string): string | null {
  const raw = html.match(/href="(magnet:\?[^"]+)"/i)?.[1];
  return raw ? unescapeEntities(raw) : null;
}

export function parseTorrentUrlFromPage(html: string): string | null {
  const match = html.match(/href="([^"?]+\.torrent)"/i);
  if (!match) return null;
  const href = match[1]!;
  if (/^https?:\/\//i.test(href)) return href;
  return new URL(href, `${HOME}/`).href;
}

async function fetchSearchHtml(query: string, opts: SearchOptions): Promise<string> {
  const res = await fetchResilient(`${HOME}/index.php`, {
    method: "POST",
    headers: {
      ...HEADERS,
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: searchBody(query),
    signal: opts.signal,
    redirect: "follow",
  });
  if (!res.ok) throw new HttpError(res.status, `Xatab returned ${res.status}`);
  return decodeResponseText(res);
}

async function fetchText(url: string, opts: SearchOptions): Promise<string> {
  const res = await fetchResilient(url, { headers: HEADERS, signal: opts.signal });
  if (!res.ok) throw new HttpError(res.status, `Xatab returned ${res.status}`);
  return decodeResponseText(res);
}

async function resolveMagnet(link: GameLink, opts: SearchOptions): Promise<TorrentResult | null> {
  try {
    const page = await fetchText(link.url, opts);
    const direct = parseMagnetFromPage(page);
    if (direct) {
      const infoHash = direct.match(/urn:btih:([a-zA-Z0-9]+)/i)?.[1]?.toLowerCase();
      if (!infoHash) return null;
      return {
        infoHash,
        name: link.title,
        sizeBytes: 0,
        seeders: 0,
        leechers: 0,
        source: "xatab",
        magnet: direct,
        added: link.added,
      };
    }

    const torrentUrl = parseTorrentUrlFromPage(page);
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
      source: "xatab",
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
  if (!q) return [];
  const html = await fetchSearchHtml(q, opts);
  const links = parseXatabSearchResults(html);
  if (links.length === 0) return [];
  return resolveAll(links, opts);
}
