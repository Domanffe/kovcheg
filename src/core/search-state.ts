import { ACTIVE_SOURCE_IDS, SOURCE_IDS, SOURCE_META } from "../sources/meta";
import type { SourceId, TorrentResult } from "../sources/types";

export interface SourceState {
  loading: boolean;
  error: string | null;
  code: string | null;
  count: number;
}

export interface ConcurrentSearchState {
  results: TorrentResult[];
  perSource: Record<SourceId, SourceState>;
  loading: boolean;
  done: number;
  total: number;
}

/**
 * A row for *every* source, not only the active ones: the record type promises a state per id,
 * so it must hold one. A disabled source is simply never loading — it reports nothing at all.
 */
export function blankPerSource(loading: boolean): Record<SourceId, SourceState> {
  const out = {} as Record<SourceId, SourceState>;
  for (const id of SOURCE_IDS) {
    out[id] = {
      loading: loading && !SOURCE_META[id].disabled,
      error: null,
      code: null,
      count: 0,
    };
  }
  return out;
}

export function idleSearchState(): ConcurrentSearchState {
  return {
    results: [],
    perSource: blankPerSource(false),
    loading: false,
    done: 0,
    total: ACTIVE_SOURCE_IDS.length,
  };
}
