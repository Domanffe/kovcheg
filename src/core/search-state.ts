import { ACTIVE_SOURCE_IDS } from "../sources/meta";
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

export function blankPerSource(loading: boolean): Record<SourceId, SourceState> {
  const out = {} as Record<SourceId, SourceState>;
  for (const id of ACTIVE_SOURCE_IDS) {
    out[id] = { loading, error: null, code: null, count: 0 };
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
