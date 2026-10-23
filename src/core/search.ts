import { cachedSearch } from "../sources/cache";
import { loadAllSources } from "../sources/registry";
import { HttpError } from "../util/net";
import type { TorrentResult } from "../sources/types";
import {
  blankPerSource,
  type ConcurrentSearchState,
} from "./search-state";

export type { ConcurrentSearchState, SourceState } from "./search-state";

function errorCode(e: unknown): string {
  if (e instanceof HttpError && e.status > 0) return `HTTP ${e.status}`;
  return "no response";
}

export function dedupeResults(list: TorrentResult[]): TorrentResult[] {
  const byHash = new Map<string, TorrentResult>();
  for (const r of list) {
    const existing = byHash.get(r.infoHash);
    if (!existing || r.seeders > existing.seeders) byHash.set(r.infoHash, r);
  }
  return [...byHash.values()];
}

/** Default ordering: healthiest first. */
export function defaultResultOrder(list: TorrentResult[]): TorrentResult[] {
  return list.sort((a, b) => {
    if (b.seeders !== a.seeders) return b.seeders - a.seeders;
    return (b.added ?? 0) - (a.added ?? 0);
  });
}

export interface SearchCallbacks {
  onUpdate: (state: ConcurrentSearchState) => void;
}

const RESULT_FLUSH_MS = 150;
/** A source that never answers must not keep the whole search in "loading" forever. */
const SOURCE_DEADLINE_MS = 25_000;

async function withDeadline<T>(label: string, work: Promise<T>): Promise<T> {
  // The loser of the race must not become an unhandled rejection.
  work.catch(() => {});
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      work,
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error(`${label} did not answer within ${SOURCE_DEADLINE_MS / 1000} s`)),
          SOURCE_DEADLINE_MS,
        );
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/** Run concurrent search across all sources; calls onUpdate as results stream in. */
export async function runConcurrentSearch(
  query: string,
  callbacks: SearchCallbacks,
  signal?: AbortSignal,
): Promise<void> {
  const sources = await loadAllSources();
  const collected: TorrentResult[] = [];
  const per = blankPerSource(true);
  let done = 0;
  const total = sources.length;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const emit = (): void => {
    callbacks.onUpdate({
      results: defaultResultOrder(dedupeResults(collected.slice())),
      perSource: { ...per },
      loading: done < total,
      done,
      total,
    });
  };

  const scheduleEmit = (): void => {
    if (done >= total) {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
      emit();
      return;
    }
    if (timer) return;
    timer = setTimeout(() => {
      timer = null;
      if (!signal?.aborted) emit();
    }, RESULT_FLUSH_MS);
  };

  emit();

  await Promise.all(
    sources.map(async (source) => {
      try {
        const res = await withDeadline(source.id, cachedSearch(source, query, { signal }));
        if (signal?.aborted) return;
        collected.push(...res);
        per[source.id] = { loading: false, error: null, code: null, count: res.length };
      } catch (e: unknown) {
        if (signal?.aborted) return;
        per[source.id] = {
          loading: false,
          error: e instanceof Error ? e.message : String(e),
          code: errorCode(e),
          count: 0,
        };
      } finally {
        if (signal?.aborted) return;
        done += 1;
        scheduleEmit();
      }
    }),
  );

  if (timer) clearTimeout(timer);
}
