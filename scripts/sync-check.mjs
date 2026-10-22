#!/usr/bin/env node
/**
 * One-way upstream sync report for kovcheg-gui.
 * Usage: node scripts/sync-check.mjs [--fetch]
 */

import { execSync } from "node:child_process";

const SYNC_PATHS = [
  "src/sources/",
  "src/util/format.ts",
  "src/util/format.test.ts",
];

const fetch = process.argv.includes("--fetch");

function run(cmd) {
  try {
    return execSync(cmd, { encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] }).trim();
  } catch (e) {
    const err = e;
    if (err.stdout) return err.stdout.trim();
    if (err.stderr?.includes("unknown revision")) return "";
    throw e;
  }
}

function hasRemote(name) {
  try {
    run(`git remote get-url ${name}`);
    return true;
  } catch {
    return false;
  }
}

function section(title) {
  console.log(`\n${title}\n${"─".repeat(title.length)}`);
}

const pathArgs = SYNC_PATHS.map((p) => `"${p}"`).join(" ");

if (!hasRemote("upstream")) {
  console.error("No upstream remote. Add: git remote add upstream https://github.com/baairon/torlink.git");
  process.exit(1);
}

if (fetch) {
  console.log("Fetching upstream…");
  run("git fetch upstream");
}

const upstreamRef = "upstream/main";
try {
  run(`git rev-parse ${upstreamRef}`);
} catch {
  console.error(`Cannot resolve ${upstreamRef}. Run: git fetch upstream`);
  process.exit(1);
}

section("Upstream commits we might want (sources zone only)");
const incoming = run(`git log HEAD..${upstreamRef} --oneline -- ${pathArgs}`);
console.log(incoming || "(none — upstream has nothing new in sync zone)");

section("Our commits in sync zone (not in upstream)");
const outgoing = run(`git log ${upstreamRef}..HEAD --oneline -- ${pathArgs}`);
console.log(outgoing || "(none)");

section("File diff: upstream/main → our HEAD (sync zone)");
const diffStat = run(`git diff --stat ${upstreamRef} HEAD -- ${pathArgs}`);
console.log(diffStat || "(identical in sync zone)");

section("Tip");
console.log("Details: docs/upstream.md");
