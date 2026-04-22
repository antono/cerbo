/**
 * Carta plugin for [[wikilink]] syntax and local assets.
 */

import type { Plugin } from 'carta-md';
import { findAndReplace, type ReplaceFunction } from 'mdast-util-find-and-replace';
import { visit } from 'unist-util-visit';
import type { Root as MdastRoot, Image } from 'mdast';
import type { Root as HastRoot, Element } from 'hast';
import { convertFileSrc } from '@tauri-apps/api/core';
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
  /** The current vault root path getter. */
  getVaultPath: () => string | null | undefined;
  /** The current page slug getter. */
  getSlug: () => string | null | undefined;
  /** Called when an asset link is clicked. */
  onOpenAsset?: (filename: string) => void;
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
                /(!?)\[\[([^\]]+)\]\]/g,
                ((_match: string, isImage: string, title: string) => {
                  const slug = titleToSlug(title);
                  
                  if (isImage) {
                    // Handle ![[...]] as an image/asset preview
                    const isActualImage = /\.(png|jpe?g|gif|svg|webp)$/i.test(title);
                    if (isActualImage) {
                      return {
                        type: 'image',
                        url: `assets/${title}`,
                        alt: title,
                        data: {
                          hProperties: {
                            'data-wikilink-asset': 'true'
                          }
                        }
                      };
                    } else {
                      // Fallback to a link if it's not an image
                      return {
                        type: 'wikilink',
                        value: title,
                        data: {
                          hName: 'a',
                          hProperties: {
                            'data-asset': 'true',
                            'data-asset-filename': title,
                            href: '#',
                            class: 'asset-link',
                          },
                          hChildren: [{ type: 'text', value: `📎 ${title}` }],
                        },
                      };
                    }
                  }

                  return {
                    type: 'wikilink',
                    value: title,
                    data: {
                      hName: 'a',
                      hProperties: {
                        'data-wikilink': 'true',
                        'data-wikilink-title': title,
                        'data-wikilink-slug': slug,
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

      // ── 2. Rehype transformer: apply resolved/broken and handle assets ───────
      {
        execution: 'sync',
        type: 'rehype',
        transform({ processor }) {
          processor.use(() => (tree: HastRoot) => {
            const pages = options.getPages();
            // Fetch once per render
            const vaultPath = options.getVaultPath();
            const currentSlug = options.getSlug();

            visit(tree, 'element', (node: Element) => {
              // Wikilinks
              if (
                node.tagName === 'a' &&
                node.properties?.['data-wikilink'] === 'true'
              ) {
                const linkSlug = node.properties['data-wikilink-slug'] as string;
                const resolved = pages.includes(linkSlug);
                node.properties['data-wikilink-resolved'] = String(resolved);
                node.properties.class = `wikilink ${resolved ? 'wikilink-resolved' : 'wikilink-broken'}`;
                node.properties.href = '#';
              }

              // Asset Links
              if (node.tagName === 'a') {
                const href = node.properties?.href as string;
                if (href && (href.startsWith('assets/') || href.startsWith('./assets/'))) {
                  const cleanPath = href.startsWith('./') ? href.slice(2) : href;
                  const encodedFilename = cleanPath.replace('assets/', '');
                  let filename = encodedFilename;
                  try {
                    filename = decodeURIComponent(encodedFilename);
                  } catch (_) {}
                  
                  node.properties['data-asset'] = 'true';
                  node.properties['data-asset-filename'] = filename;
                  node.properties.href = '#';
                }
              }

              // Images
              if (node.tagName === 'img') {
                const src = node.properties?.src as string;
                if (src && (src.startsWith('assets/') || src.startsWith('./assets/'))) {
                  if (vaultPath && currentSlug) {
                    const cleanPath = src.startsWith('./') ? src.slice(2) : src;
                    const encodedFilename = cleanPath.replace('assets/', '');
                    let filename = encodedFilename;
                    try {
                      filename = decodeURIComponent(encodedFilename);
                    } catch (_) {}
                    
                    const base = vaultPath.endsWith('/') ? vaultPath.slice(0, -1) : vaultPath;
                    const fullPath = `${base}/${currentSlug}/assets/${filename}`;
                    node.properties.src = convertFileSrc(fullPath);
                  }
                }
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
  options: Pick<WikilinkPluginOptions, 'onNavigate' | 'onCreate' | 'onOpenAsset'>,
): () => void {
  function handler(ev: MouseEvent) {
    const target = (ev.target as HTMLElement).closest('a');
    if (!target) return;
    
    // Handle Wikilinks
    if (target.hasAttribute('data-wikilink')) {
      ev.preventDefault();
      const slug = target.getAttribute('data-wikilink-slug') ?? '';
      const title = target.getAttribute('data-wikilink-title') ?? '';
      const resolved = target.getAttribute('data-wikilink-resolved') === 'true';
      if (resolved) {
        options.onNavigate(slug);
      } else {
        options.onCreate(title);
      }
      return;
    }

    // Handle Assets
    if (target.hasAttribute('data-asset')) {
      ev.preventDefault();
      const filename = target.getAttribute('data-asset-filename');
      if (filename && options.onOpenAsset) {
        options.onOpenAsset(filename);
      }
      return;
    }
  }
  previewEl.addEventListener('click', handler);
  return () => previewEl.removeEventListener('click', handler);
}
