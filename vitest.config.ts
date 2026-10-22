import os from "node:os";
import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    env: {
      KOVCHEG_STATE_DIR: path.join(os.tmpdir(), "kovcheg-test-state"),
    },
  },
});
