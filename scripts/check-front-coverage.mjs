#!/usr/bin/env node
// Cross-checks that every `front`-tagged edge case in EDGE_CASES.md is covered
// by a Playwright test whose title contains the case id in brackets, e.g.
//   test('[FE-0123] ...')
//
// Prints the count of front cases, how many are covered, and the list of
// uncovered ids. Exit 0 only when every front case has a matching test
// ("0 cas front sans test").

import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const MD_PATH = resolve(ROOT, "EDGE_CASES.md");
const E2E_DIR = resolve(ROOT, "frontend", "tests", "e2e");

// ---- 1. collect front-tagged case ids from EDGE_CASES.md ----
// Skip fenced code blocks so the format example is not parsed.
const raw = readFileSync(MD_PATH, "utf8");
const lines = [];
{
  let inFence = false;
  for (const l of raw.split(/\r?\n/)) {
    if (/^\s*```/.test(l)) {
      inFence = !inFence;
      lines.push("");
      continue;
    }
    lines.push(inFence ? "" : l);
  }
}

const frontIds = [];
{
  let i = 0;
  while (i < lines.length) {
    if (lines[i].trim() === "@case") {
      const obj = {};
      let j = i + 1;
      while (j < lines.length && lines[j].trim() !== "@end") {
        const m = lines[j].match(/^\s*([^:]+):\s*(.*)$/);
        if (m) obj[m[1].trim()] = m[2].trim();
        j++;
      }
      if (obj.tag === "front" && obj.id) frontIds.push(obj.id);
      i = j + 1;
    } else {
      i++;
    }
  }
}
const frontSet = new Set(frontIds);

// ---- 2. collect ids referenced in test titles ----
function walk(dir) {
  let out = [];
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const e of entries) {
    const p = join(dir, e);
    const st = statSync(p);
    if (st.isDirectory()) out = out.concat(walk(p));
    else if (/\.(spec|test)\.(ts|js|mjs)$/.test(e)) out.push(p);
  }
  return out;
}

// Only ids that appear inside a test()/test.xxx() title string count as covered.
const TITLE_RE = /\btest\s*(?:\.\w+)?\s*\(\s*([`'"])([\s\S]*?)\1/g;
const coveredInTitles = new Set();
const testFiles = walk(E2E_DIR);
for (const f of testFiles) {
  const src = readFileSync(f, "utf8");
  let m;
  while ((m = TITLE_RE.exec(src)) !== null) {
    const title = m[2];
    const ids = title.match(/FE-\d{4}/g) || [];
    for (const id of ids) coveredInTitles.add(id);
  }
}

// ---- 3. report ----
const uncovered = frontIds.filter((id) => !coveredInTitles.has(id)).sort();
const covered = frontIds.filter((id) => coveredInTitles.has(id));
// ids tagged in tests but not present as a front case (stale tags)
const stale = [...coveredInTitles].filter((id) => !frontSet.has(id)).sort();

const sep = "=".repeat(60);
console.log(sep);
console.log("Couverture des cas front (EDGE_CASES.md × tests Playwright)");
console.log(sep);
console.log(`Fichiers de test scannés : ${testFiles.length}`);
console.log(`Cas front (tag: front)   : ${frontIds.length}`);
console.log(`Cas front couverts       : ${covered.length}`);
console.log(`Cas front SANS test      : ${uncovered.length}`);
if (stale.length)
  console.log(`(tags FE-#### sans cas front correspondant : ${stale.join(", ")})`);

if (uncovered.length) {
  console.log("\n--- CAS FRONT SANS TEST ---");
  for (const id of uncovered) console.log("  - " + id);
}

console.log("\n" + sep);
if (uncovered.length === 0) {
  console.log("0 cas front sans test");
} else {
  console.log(`${uncovered.length} cas front sans test`);
}
console.log(sep);
process.exit(uncovered.length === 0 ? 0 : 1);
