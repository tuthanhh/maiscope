#!/usr/bin/env node
// Fails when a relative Markdown link or image path does not resolve on disk.
// External URLs (http/https/mailto) and pure anchors are skipped: they fail for
// reasons unrelated to this repository and would make CI flaky.
//
// Fenced code blocks (``` or ~~~, with an optional info string, closed by a
// fence of at least the same length/character) are skipped entirely: they
// quote example content — sample syntax, other files' bodies, fixture text —
// not references this document makes itself. Inline single-backtick code
// spans are stripped for the same reason before link-matching a line.

import { readdir, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, dirname, resolve, extname } from "node:path";

const IGNORED_DIRS = new Set(["node_modules", "target", ".git", "dist"]);
const LINK = /\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;
const FENCE_OPEN = /^ {0,3}(`{3,}|~{3,})/;
const FENCE_CLOSE = /^ {0,3}(`+|~+)\s*$/;

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

// Strips lines that fall inside fenced code blocks (both open/close fence
// markers themselves and everything between them), returning the same number
// of lines so line numbers stay accurate — skipped lines become "".
//
// Also reports a fence left open at end of file. That case is not cosmetic:
// every line after the stray fence is blanked, so the rest of the document
// is never link-checked and the run still passes. Callers must fail on it.
function stripFencedBlocks(lines) {
  const out = [];
  let fence = null; // { char: '`' | '~', len: number, line: number }

  for (const [index, line] of lines.entries()) {
    if (fence) {
      const close = line.match(FENCE_CLOSE);
      if (close && close[1][0] === fence.char && close[1].length >= fence.len) {
        fence = null;
      }
      out.push(""); // fence content (and the closing marker) is never scanned
      continue;
    }

    const open = line.match(FENCE_OPEN);
    if (open) {
      fence = { char: open[1][0], len: open[1].length, line: index + 1 };
      out.push(""); // opening marker line itself is not scanned
      continue;
    }

    out.push(line);
  }

  return { lines: out, unterminated: fence ? fence.line : null };
}

const root = process.argv[2] ?? ".";
const failures = [];

for (const file of await markdownFiles(root)) {
  const body = await readFile(file, "utf8");
  const { lines, unterminated } = stripFencedBlocks(body.split("\n"));

  if (unterminated !== null) {
    failures.push(
      `${file}:${unterminated}: code fence opened here is never closed — ` +
        `every line below it went unchecked`,
    );
  }

  for (const [index, rawLine] of lines.entries()) {
    // Inline code spans (`like this`) quote example text too; drop them
    // before matching so a link mentioned inside one isn't treated as real.
    const line = rawLine.replace(/`[^`]*`/g, "");

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
  console.error(`${failures.length} problem(s):\n`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}

console.log("All relative Markdown links resolve.");
