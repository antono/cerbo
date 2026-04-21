/**
 * Carta plugin for [[wikilink]] syntax.
 *
 * Features:
 * - Tokenizes [[Page Title]] in markdown (remark pass)
 * - Renders as <a data-wikilink> with resolved/broken class (rehype pass)
 * - Exposes findAndReplace helper for reactive page list updates
 * - Provides autocomplete component injected into Carta input
 */

import type { Plugin } from 'carta-md';
import { findAndReplace, type ReplaceFunction } from 'mdast-util-find-and-replace';
import { visit } from 'unist-util-visit';
import type { Root as MdastRoot, PhrasingContent, Text } from 'mdast';
import type { Root as HastRoot, Element } from 'hast';
import WikilinkAutocomplete from './WikilinkAutocomplete.svelte';

// ── Custom mdast node ─────────────────────────────────────────────────────────

interface WikilinkNode {
  type: 'wikilink';
  value: string; // original title text
  data: {
    hName: string;
    hProperties: Record<string, string>;
    hChildren: Array<{ type: 'text'; value: string }>;
  };
}

declare module 'mdast' {
  interface StaticPhrasingContentMap {
    wikilink: WikilinkNode;
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Derive a slug from a page title (mirrors Rust derive_slug logic). */
function titleToSlug(title: string): string {
  return title
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '') // strip diacritics
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/[\s_]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
}

// ── Plugin factory ────────────────────────────────────────────────────────────

export interface WikilinkPluginOptions {
  /** Reactive getter — called each render cycle to get the current slug list. */
  getPages: () => string[];
  /** Called when the user clicks a resolved wikilink. */
  onNavigate: (slug: string) => void;
  /** Called when the user clicks a broken wikilink (offer to create). */
  onCreate: (title: string) => void;
}

export function wikilinkPlugin(options: WikilinkPluginOptions): Plugin {
  return {
    // ── 1. Remark transformer: parse [[...]] into wikilink nodes ──────────────
    transformers: [
      {
        execution: 'sync',
        type: 'remark',
        transform({ processor }) {
          processor.use(() => (tree: MdastRoot) => {
            findAndReplace(tree, [
              [
                /\[\[([^\]]+)\]\]/g,
                ((_match: string, title: string) => {
                  const slug = titleToSlug(title);
                  return {
                    type: 'wikilink',
                    value: title,
                    data: {
                      hName: 'a',
                      hProperties: {
                        'data-wikilink': 'true',
                        'data-wikilink-title': title,
                        'data-wikilink-slug': slug,
                        // resolved/broken is set in the rehype pass
                        class: 'wikilink',
                      },
                      hChildren: [{ type: 'text', value: title }],
                    },
                  };
                }) as unknown as ReplaceFunction,
              ],
            ]);
          });
        },
      },

      // ── 2. Rehype transformer: apply resolved/broken classes ────────────────
      {
        execution: 'sync',
        type: 'rehype',
        transform({ processor }) {
          processor.use(() => (tree: HastRoot) => {
            const pages = options.getPages();
            visit(tree, 'element', (node: Element) => {
              if (
                node.tagName === 'a' &&
                node.properties?.['data-wikilink'] === 'true'
              ) {
                const slug = node.properties['data-wikilink-slug'] as string;
                const resolved = pages.includes(slug);
                node.properties['data-wikilink-resolved'] = String(resolved);
                node.properties.class = `wikilink ${resolved ? 'wikilink-resolved' : 'wikilink-broken'}`;
                // Prevent href navigation; clicks handled via JS
                node.properties.href = '#';
              }
            });
          });
        },
      },
    ],

    // ── 3. Autocomplete component injected into the editor input area ─────────
    components: [
      {
        component: WikilinkAutocomplete as any,
        props: { getPages: options.getPages } as any,
        parent: 'input',
      },
    ],
  };
}

/** Attach a click listener to the preview container to handle wikilink clicks. */
export function attachPreviewClickHandler(
  previewEl: HTMLElement,
  options: Pick<WikilinkPluginOptions, 'onNavigate' | 'onCreate'>,
): () => void {
  function handler(ev: MouseEvent) {
    const target = (ev.target as HTMLElement).closest('[data-wikilink]');
    if (!target) return;
    ev.preventDefault();
    const slug = target.getAttribute('data-wikilink-slug') ?? '';
    const title = target.getAttribute('data-wikilink-title') ?? '';
    const resolved = target.getAttribute('data-wikilink-resolved') === 'true';
    if (resolved) {
      options.onNavigate(slug);
    } else {
      options.onCreate(title);
    }
  }
  previewEl.addEventListener('click', handler);
  return () => previewEl.removeEventListener('click', handler);
}
