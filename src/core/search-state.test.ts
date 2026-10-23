import { describe, expect, it } from "vitest";
import { blankPerSource } from "./search-state";
import { ACTIVE_SOURCE_IDS, SOURCE_IDS, SOURCE_META } from "../sources/meta";

describe("blankPerSource", () => {
  it("holds a state for every source, so the record type is not a promise it breaks", () => {
    const per = blankPerSource(false);
    for (const id of SOURCE_IDS) {
      expect(per[id], id).toBeDefined();
      expect(per[id].count).toBe(0);
    }
  });

  it("never reports a disabled source as loading", () => {
    const per = blankPerSource(true);
    for (const id of SOURCE_IDS) {
      expect(per[id].loading, id).toBe(!SOURCE_META[id].disabled);
    }
    expect(ACTIVE_SOURCE_IDS.every((id) => per[id])).toBe(true);
  });
});
