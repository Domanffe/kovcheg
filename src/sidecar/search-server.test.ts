import { describe, it, expect } from "vitest";
import { ACTIVE_SOURCE_IDS } from "../sources/meta";
import { startSearchServer } from "./search-server.js";

describe("search sidecar", () => {
  it("serves health and sources", async () => {
    const { port, close } = await startSearchServer(0);
    try {
      const health = await fetch(`http://127.0.0.1:${port}/health`);
      expect(health.ok).toBe(true);
      const sources = await fetch(`http://127.0.0.1:${port}/sources`);
      const body = (await sources.json()) as { sources: unknown[] };
      expect(body.sources.length).toBe(ACTIVE_SOURCE_IDS.length);
    } finally {
      close();
    }
  });

  it("streams search results over SSE", async () => {
    const { port, close } = await startSearchServer(0);
    try {
      const res = await fetch(`http://127.0.0.1:${port}/search?q=__sidecar-test__`);
      expect(res.ok).toBe(true);
      const reader = res.body!.getReader();
      const { value } = await reader.read();
      const chunk = new TextDecoder().decode(value ?? new Uint8Array());
      expect(chunk).toContain("data:");
      await reader.cancel();
    } finally {
      close();
    }
  });
});
