/**
 * Carta plugin for HackMD annotations: `[Berlin]{schema:Place}` renders as a
 * styled mention whose tooltip shows the type as written and as a full IRI
 * (design D3). Mentions are never links.
 */

import type { Plugin } from 'carta-md';
import { findAndReplace, type RegExpMatchObject, type ReplaceFunction } from 'mdast-util-find-and-replace';
import { visit } from 'unist-util-visit';
import type { Root as MdastRoot, Parent, PhrasingContent, Text } from 'mdast';

// ── Custom mdast node ─────────────────────────────────────────────────────────

interface MentionNode {
  type: 'mention';
  value: string; // the mentioned text
  data: {
    hName: string;
    hProperties: Record<string, string>;
    hChildren: Array<{ type: 'text'; value: string }>;
  };
}

declare module 'mdast' {
  interface StaticPhrasingContentMap {
    mention: MentionNode;
  }
}

// ── Syntax ────────────────────────────────────────────────────────────────────

// Only an identifier-shaped prefix counts, so task lists, links and other
// bracket syntax next to a mention stay untouched.
const PREFIX = '[A-Za-z_][\\w.-]*';
const LOCAL = '[^\\s{}\\[\\]]+';

/** `[text]{prefix:Type}` inside one text node. */
const MENTION_RE = new RegExp(`\\[([^\\[\\]\\n]+)\\]\\{(${PREFIX}):(${LOCAL})\\}`, 'g');

/** `{prefix:Type}` right after a `[text]` that markdown parsed as a link reference. */
const TYPE_AFTER_REFERENCE_RE = new RegExp(`^\\{(${PREFIX}):(${LOCAL})\\}`);

// ── Resolution ────────────────────────────────────────────────────────────────

/** `prefix → namespace`, as returned by `prefix_map_get`. */
export type PrefixMap = Record<string, string>;

export type MentionType =
  | { status: 'resolved'; written: string; iri: string }
  | { status: 'unresolved'; written: string; prefix: string }
  // The vault's prefix map has not loaded (or failed to): no warning then.
  | { status: 'unknown'; written: string };

export function resolveMentionType(
  prefixes: PrefixMap | null,
  prefix: string,
  local: string,
): MentionType {
  const written = `${prefix}:${local}`;
  if (!prefixes) return { status: 'unknown', written };
  const namespace = Object.prototype.hasOwnProperty.call(prefixes, prefix) ? prefixes[prefix] : undefined;
  if (namespace === undefined) return { status: 'unresolved', written, prefix };
  return { status: 'resolved', written, iri: namespace + local };
}

function mentionNode(text: string, type: MentionType): MentionNode {
  const hProperties: Record<string, string> = {
    class: type.status === 'unresolved' ? 'mention unresolved' : 'mention',
    'data-mention-type': type.written,
  };
  if (type.status === 'resolved') {
    hProperties['data-mention-iri'] = type.iri;
    hProperties.title = `${type.written}\n${type.iri}`;
  } else if (type.status === 'unresolved') {
    hProperties.title = `${type.written}\nUnknown prefix "${type.prefix}" in this vault`;
  } else {
    hProperties.title = type.written;
  }
  return {
    type: 'mention',
    value: text,
    data: { hName: 'span', hProperties, hChildren: [{ type: 'text', value: text }] },
  };
}

const ASCII_PUNCTUATION = /[!-/:-@[-`{-~]/;
const CHARACTER_REFERENCE = /^&(?:#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|[A-Za-z][A-Za-z0-9]{0,31});/;

/**
 * Whether the character at `index` of text node `node`'s value was written
 * backslash-escaped in `source`. The parser drops the backslash from the value,
 * so `\[Berlin]{schema:Place}` arrives as `[Berlin]{schema:Place}`; walking the
 * source alongside the value finds it again. False when the node has no
 * position to walk from.
 */
export function escapedInSource(source: string, node: Text, index: number): boolean {
  let s = node.position?.start.offset;
  const end = node.position?.end.offset;
  if (s === undefined || end === undefined) return false;
  const value = node.value;
  for (let v = 0; v <= index && s < end; v++) {
    const escaped = source[s] === '\\' && ASCII_PUNCTUATION.test(source[s + 1] ?? '') && source[s + 1] === value[v];
    if (v === index) return escaped;
    if (escaped) {
      s += 2;
    } else if (source[s] === '&' && value[v] !== '&') {
      // A character reference such as `&amp;` became one character.
      s += CHARACTER_REFERENCE.exec(source.slice(s))?.[0].length ?? 1;
    } else {
      s += 1;
    }
  }
  return false;
}

function plainText(node: PhrasingContent): string {
  if ('value' in node && typeof node.value === 'string') return node.value;
  if ('children' in node) return (node.children as PhrasingContent[]).map(plainText).join('');
  return '';
}

/** Remark transform: turn annotations into `mention` nodes. */
export function remarkMentions(getPrefixes: () => PrefixMap | null) {
  return (tree: MdastRoot, file?: { value?: unknown }) => {
    const prefixes = getPrefixes();
    const source = typeof file?.value === 'string' ? file.value : '';

    // `[Berlin]` matching a link definition is parsed as a link reference before
    // any plugin runs; take it back when a type follows it.
    visit(tree, 'linkReference', (node, index, parent: Parent | undefined) => {
      if (!parent || index === undefined) return;
      const next = parent.children[index + 1];
      if (!next || next.type !== 'text') return;
      const match = TYPE_AFTER_REFERENCE_RE.exec((next as Text).value);
      if (!match) return;
      const text = (node.children as PhrasingContent[]).map(plainText).join('');
      const mention = mentionNode(text, resolveMentionType(prefixes, match[1], match[2]));
      const rest = (next as Text).value.slice(match[0].length);
      // Keep the text after the type; drop the text node once it is empty.
      (next as Text).value = rest;
      parent.children.splice(index, rest ? 1 : 2, mention as never);
      return index + 1;
    });

    findAndReplace(tree, [
      [
        MENTION_RE,
        ((_match: string, text: string, prefix: string, local: string, match: RegExpMatchObject) => {
          // `\[Berlin]{schema:Place}` shows the syntax literally.
          const node = match.stack[match.stack.length - 1] as Text;
          if (source && escapedInSource(source, node, match.index)) return false;
          return mentionNode(text, resolveMentionType(prefixes, prefix, local));
        }) as unknown as ReplaceFunction,
      ],
    ]);
  };
}

// ── Plugin factory ────────────────────────────────────────────────────────────

export interface MentionPluginOptions {
  /** Reactive getter — the open vault's prefix map, or null before it loads. */
  getPrefixes: () => PrefixMap | null;
}

export function mentionPlugin(options: MentionPluginOptions): Plugin {
  return {
    transformers: [
      {
        execution: 'sync',
        type: 'remark',
        transform({ processor }) {
          processor.use(() => remarkMentions(options.getPrefixes));
        },
      },
    ],
  };
}
