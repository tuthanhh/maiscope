#!/usr/bin/env node
// Fails when a relative Markdown link or image path does not resolve on disk.
// External URLs (http/https/mailto) and pure anchors are skipped: they fail for
// reasons unrelated to this repository and would make CI flaky.

import { readdir, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, dirname, resolve, extname } from "node:path";

const IGNORED_DIRS = new Set(["node_modules", "target", ".git", "dist"]);
const LINK = /\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;

async function markdownFiles(dir) {
  const found = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    if (entry.name.startsWith(".") && entry.name !== ".github") continue;
    if (IGNORED_DIRS.has(entry.name)) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) found.push(...(await markdownFiles(full)));
    else if (extname(entry.name) === ".md") found.push(full);
  }
  return found;
}

const root = process.argv[2] ?? ".";
const failures = [];

for (const file of await markdownFiles(root)) {
  const body = await readFile(file, "utf8");
  const lines = body.split("\n");

  for (const [index, line] of lines.entries()) {
    for (const match of line.matchAll(LINK)) {
      const target = match[1];
      if (/^(https?:|mailto:|#)/.test(target)) continue;

      const path = target.split("#")[0];
      if (path === "") continue;

      if (!existsSync(resolve(dirname(file), path))) {
        failures.push(`${file}:${index + 1}: broken link -> ${target}`);
      }
    }
  }
}

if (failures.length > 0) {
  console.error(`${failures.length} broken reference(s):\n`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}

console.log("All relative Markdown links resolve.");
