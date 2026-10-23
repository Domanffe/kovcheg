import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import type { ConcurrentSearchState } from "../../core/search-state";
import { idleSearchState } from "../../core/search-state";

import { isTauri } from "../util/tauri";

export function useSearchPort(): number {
  const [port, setPort] = useState(() => {
    if (typeof window === "undefined") return 3847;
    const p = new URLSearchParams(window.location.search).get("searchPort");
    return p ? Number(p) : 3847;
  });

  useEffect(() => {
    if (!isTauri()) return;
    void invoke<number>("get_search_port").then(setPort).catch(() => {});
  }, []);

  return port;
}

/** Search runs only when `activeQuery` is set (deferred browse — no empty query on mount). */
export function useSearch(activeQuery: string | null, port: number): ConcurrentSearchState {
  const [state, setState] = useState<ConcurrentSearchState>(idleSearchState);
  const ctrlRef = useRef<AbortController | null>(null);

  const run = useCallback(
    (query: string) => {
      ctrlRef.current?.abort();
      const ctrl = new AbortController();
      ctrlRef.current = ctrl;
      setState({ ...idleSearchState(), loading: true });

      const url = `http://127.0.0.1:${port}/search?q=${encodeURIComponent(query)}`;
      const es = new EventSource(url);

      es.onmessage = (ev) => {
        try {
          setState(JSON.parse(ev.data) as ConcurrentSearchState);
        } catch {
          // A malformed frame must not tear down the stream; the next one replaces the state.
        }
      };
      es.addEventListener("done", () => es.close());
      es.addEventListener("error", () => es.close());
      ctrl.signal.addEventListener("abort", () => es.close());
    },
    [port],
  );

  useEffect(() => {
    if (activeQuery === null) {
      ctrlRef.current?.abort();
      setState(idleSearchState());
      return;
    }
    if (activeQuery.trim() === "") return;
    run(activeQuery);
    return () => ctrlRef.current?.abort();
  }, [activeQuery, run]);

  return state;
}
