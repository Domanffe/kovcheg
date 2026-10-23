import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Source, TorrentResult } from "../sources/types";
import {
  dedupeResults,
  defaultResultOrder,
  runConcurrentSearch,
} from "./search";

const base = (hash: string, seeders: number, added = 0): TorrentResult => ({
  infoHash: hash,
  name: hash,
  sizeBytes: 0,
  seeders,
  leechers: 0,
  source: "yts",
  magnet: `magnet:?xt=urn:btih:${hash}`,
  added,
});

function mockSource(id: Source["id"], label?: string): Source {
  return {
    id,
    label: label ?? id,
    group: "Movies",
    homepage: "",
    reportsHealth: true,
    search: async () => [],
  };
}

describe("dedupeResults", () => {
  it("keeps the entry with more seeders for the same hash", () => {
    const out = dedupeResults([
      base("abc", 10),
      base("abc", 25),
      base("def", 5),
    ]);
    expect(out).toHaveLength(2);
    expect(out.find((r) => r.infoHash === "abc")?.seeders).toBe(25);
  });
});

describe("defaultResultOrder", () => {
  it("sorts by seeders then added date", () => {
    const out = defaultResultOrder([
      base("a", 5, 100),
      base("b", 10, 50),
      base("c", 10, 200),
    ]);
    expect(out.map((r) => r.infoHash)).toEqual(["c", "b", "a"]);
  });
});

vi.mock("../sources/registry", () => ({
  loadAllSources: vi.fn(),
}));

vi.mock("../sources/cache", () => ({
  cachedSearch: vi.fn(),
}));

import { loadAllSources } from "../sources/registry";
import { cachedSearch } from "../sources/cache";

const mockLoad = vi.mocked(loadAllSources);
const mockCached = vi.mocked(cachedSearch);

beforeEach(() => {
  mockLoad.mockReset();
  mockCached.mockReset();
});

describe("runConcurrentSearch", () => {
  it("streams updates and marks loading complete", async () => {
    mockLoad.mockResolvedValue([mockSource("yts"), mockSource("eztv")]);
    mockCached
      .mockResolvedValueOnce([base("aaa", 3)])
      .mockResolvedValueOnce([base("bbb", 7)]);

    const updates: unknown[] = [];
    await runConcurrentSearch("test", {
      onUpdate: (s) => updates.push(s),
    });

    const last = updates.at(-1) as {
      loading: boolean;
      done: number;
      total: number;
      results: TorrentResult[];
    };
    expect(last.loading).toBe(false);
    expect(last.done).toBe(2);
    expect(last.total).toBe(2);
    expect(last.results).toHaveLength(2);
  });

  it("records per-source errors without aborting others", async () => {
    mockLoad.mockResolvedValue([mockSource("yts")]);
    mockCached.mockRejectedValueOnce(new Error("offline"));

    const updates: unknown[] = [];
    await runConcurrentSearch("x", { onUpdate: (s) => updates.push(s) });
    const last = updates.at(-1) as { perSource: Record<string, { error: string | null }> };
    expect(last.perSource.yts?.error).toBe("offline");
  });

  it("gives up on a source that never answers", async () => {
    vi.useFakeTimers();
    try {
      mockLoad.mockResolvedValue([mockSource("yts"), mockSource("eztv")]);
      mockCached.mockImplementation((source) =>
        source.id === "yts"
          ? Promise.resolve([base("aaa", 3)])
          : new Promise(() => {}), // never settles
      );

      const updates: unknown[] = [];
      const run = runConcurrentSearch("hang", { onUpdate: (s) => updates.push(s) });
      await vi.advanceTimersByTimeAsync(30_000);
      await run;

      const last = updates.at(-1) as {
        loading: boolean;
        done: number;
        perSource: Record<string, { error: string | null; loading: boolean }>;
        results: TorrentResult[];
      };
      expect(last.loading).toBe(false);
      expect(last.done).toBe(2);
      expect(last.perSource.eztv?.loading).toBe(false);
      expect(last.perSource.eztv?.error).toMatch(/did not answer/);
      expect(last.results).toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });
});
