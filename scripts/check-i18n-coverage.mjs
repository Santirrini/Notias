#!/usr/bin/env node
/**
 * i18n coverage check — fail (exit 1) if any message key is missing in any
 * locale, or if any locale has a key the others don't.
 *
 * Compares flat keys (the JSON first level past $schema) across all files
 * matching `./messages/*.json`. Run as part of CI / pre-push.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const messagesDir = join(__dirname, "..", "messages");

const files = readdirSync(messagesDir).filter((f) => f.endsWith(".json"));
if (files.length < 2) {
  console.warn(
    `[i18n:check] Only ${files.length} locale file(s) found; nothing to compare. Skipping.`,
  );
  process.exit(0);
}

/** @type {Record<string, Set<string>>} */
const keysByLocale = {};
for (const file of files) {
  const locale = file.replace(/\.json$/, "");
  const raw = JSON.parse(readFileSync(join(messagesDir, file), "utf8"));
  const keys = new Set();
  for (const k of Object.keys(raw)) {
    if (k.startsWith("$")) continue;
    keys.add(k);
  }
  keysByLocale[locale] = keys;
}

const referenceLocale = Object.keys(keysByLocale)[0];
const referenceKeys = keysByLocale[referenceLocale];

let missing = 0;
let extra = 0;

for (const locale of Object.keys(keysByLocale)) {
  if (locale === referenceLocale) continue;
  const theirKeys = keysByLocale[locale];
  for (const k of referenceKeys) {
    if (!theirKeys.has(k)) {
      console.error(`  ✗ ${locale}.json is missing key "${k}"`);
      missing++;
    }
  }
  for (const k of theirKeys) {
    if (!referenceKeys.has(k)) {
      console.error(`  ✗ ${locale}.json has extra key "${k}" (not in ${referenceLocale})`);
      extra++;
    }
  }
}

if (missing === 0 && extra === 0) {
  console.log(
    `[i18n:check] OK — ${referenceKeys.size} keys aligned across ${Object.keys(keysByLocale).length} locales: ${Object.keys(keysByLocale).join(", ")}`,
  );
  process.exit(0);
} else {
  console.error(
    `\n[i18n:check] FAIL — ${missing} missing, ${extra} extra keys across locales.`,
  );
  process.exit(1);
}
