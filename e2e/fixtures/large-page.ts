// A 137 KB markdown page for typing-latency checks (spec
// `editor-input-performance`). Deterministic and ASCII-only, so its length in
// characters equals its size in bytes. It mixes the syntax the editor plugins
// and the shiki overlay care about: headings, emphasis, inline code, links,
// cerbo links, lists, task lists, quotes, tables and fenced code.
//
//   bun e2e/fixtures/large-page.ts               # print the markdown
//   bun e2e/fixtures/large-page.ts <vault-dir>   # add it to a vault as a page
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export const LARGE_PAGE_UUID = '00000000-0000-4000-8000-000000137137';
export const LARGE_PAGE_TITLE = 'Large Page';
export const LARGE_PAGE_BYTES = 137 * 1024;

function section(i: number): string {
  return [
    `## Section ${i}`,
    '',
    `Paragraph ${i} has **bold**, _italic_, \`inline code\` and a [link](https://example.com/${i}).`,
    `It also points at the [index](cerbo://objects/ba079793-b2ff-4ca6-ba88-8764ce6668c3) page.`,
    '',
    `- item ${i}.1`,
    `- item ${i}.2`,
    `  - nested ${i}.2.1`,
    '',
    `- [ ] open task ${i}`,
    `- [x] done task ${i}`,
    '',
    `> Quote ${i}: keep typing fast.`,
    '',
    '| key | value |',
    '| --- | ----- |',
    `| row | ${i} |`,
    '',
    '```ts',
    `const answer${i} = ${i} * 137;`,
    `console.log(answer${i});`,
    '```',
    '',
    '',
  ].join('\n');
}

/** Markdown of exactly `bytes` ASCII characters, starting with the title H1. */
export function largePageMarkdown(bytes = LARGE_PAGE_BYTES): string {
  const parts = [`# ${LARGE_PAGE_TITLE}\n\n`];
  let length = parts[0].length;
  for (let i = 1; length < bytes; i++) {
    const s = section(i);
    parts.push(s);
    length += s.length;
  }
  return parts.join('').slice(0, bytes - 1) + '\n';
}

/** Writes the page as a cerbo object into `vaultDir` and returns its directory. */
export function writeLargePage(vaultDir: string): string {
  const dir = join(vaultDir, '.cerbo', 'objects', LARGE_PAGE_UUID);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'page.md'), largePageMarkdown());
  writeFileSync(join(dir, 'annotations.ttl'), '');
  writeFileSync(
    join(dir, 'meta.ttl'),
    [
      '@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .',
      '@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .',
      '@prefix schema: <https://schema.org/> .',
      '@prefix cerbo: <cerbo://ontology/> .',
      `<cerbo://objects/${LARGE_PAGE_UUID}> a schema:NoteDigitalDocument ;`,
      `\tschema:name "${LARGE_PAGE_TITLE}" ;`,
      '\tschema:dateCreated "2026-11-07T00:00:00+00:00"^^xsd:dateTime ;',
      '\tschema:dateModified "2026-11-07T00:00:00+00:00"^^xsd:dateTime ;',
      '\tcerbo:slug "large-page" .',
      '',
    ].join('\n'),
  );
  return dir;
}

if ((import.meta as { main?: boolean }).main) {
  const vaultDir = process.argv[2];
  if (vaultDir) {
    console.log(writeLargePage(vaultDir));
  } else {
    process.stdout.write(largePageMarkdown());
  }
}
