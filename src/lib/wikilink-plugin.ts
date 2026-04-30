/**
 * Carta plugin for [[wikilink]] syntax and local assets.
 */

import type { Plugin } from 'carta-md';
import { findAndReplace, type ReplaceFunction } from 'mdast-util-find-and-replace';
import { visit } from 'unist-util-visit';
import type { Root as MdastRoot, Image } from 'mdast';
import type { Root as HastRoot, Element } from 'hast';
import { convertFileSrc } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
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

export function previewTaskListPlugin(): Plugin {
  return {
    transformers: [
      {
        execution: 'sync',
        type: 'rehype',
        transform({ processor }) {
          processor.use(() => (tree: HastRoot) => {
            let taskIndex = 0;

            visit(tree, 'element', (node: Element) => {
              if (node.tagName !== 'li') return;

              const className = node.properties?.class;
              const classValue = Array.isArray(className) ? className.join(' ') : String(className ?? '');
              const isTaskItem = classValue.includes('task-list-item') || classValue.includes('contains-task-list');
              if (!isTaskItem) return;

              node.properties['data-task-list-item'] = 'true';
              node.properties['data-task-list-index'] = String(taskIndex);
              taskIndex += 1;

              visit(node, 'element', (child: Element) => {
                if (child.tagName !== 'input') return;
                const type = child.properties?.type;
                if (type !== 'checkbox') return;

                if ('disabled' in child.properties) {
                  delete child.properties.disabled;
                }
                child.properties.tabIndex = 0;
                child.properties.value = String(taskIndex);
                child.properties['data-task-list-checkbox'] = 'true';
                child.properties['data-task-list-index'] = node.properties['data-task-list-index'];
                child.properties.class = 'task-list-checkbox';
              });
            });
          });
        },
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
    
    // Handle External Links
    const href = target.getAttribute('href');
    if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
      ev.preventDefault();
      openUrl(href).catch((err) => console.error('Failed to open external URL:', err));
      return;
    }

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

export function attachTaskListClickHandler(
  previewEl: HTMLElement,
  options: { onToggleTask: (index: number, checked: boolean) => void },
): () => void {
  const itemHandlers = new Map<HTMLElement, (ev: MouseEvent) => void>();
  const checkboxHandlers = new Map<HTMLInputElement, (ev: Event) => void>();

  const syncHandlers = () => {
    const items = Array.from(previewEl.querySelectorAll('.task-list-item')) as HTMLElement[];
    const next = new Set(items);

    console.debug('[task-list] syncing inputs', {
      count: items.length,
      previewEl,
    });

    for (const item of items) {
      if (itemHandlers.has(item)) continue;

      const input =
        (item.querySelector(':scope > input[type="checkbox"]') as HTMLInputElement | null) ??
        (item.querySelector('input[type="checkbox"]') as HTMLInputElement | null);
      if (!input) continue;

      if (input.disabled) {
        console.debug('[task-list] enabling disabled input', { input });
        input.disabled = false;
        input.removeAttribute('disabled');
      }

      if (!checkboxHandlers.has(input)) {
        const checkboxHandler = (ev: Event) => {
          const index = Number(input.value);
          if (Number.isNaN(index)) return;

          ev.stopPropagation();
          if ('stopImmediatePropagation' in ev) ev.stopImmediatePropagation();

          console.debug('[task-list] checkbox change', {
            type: ev.type,
            index,
            checked: input.checked,
            value: input.value,
            item,
            input,
          });

          options.onToggleTask(index, input.checked);
        };

        checkboxHandlers.set(input, checkboxHandler);
        input.addEventListener('change', checkboxHandler);
        console.debug('[task-list] change handler attached to checkbox', input);
      }

      const handler = (ev: MouseEvent) => {
        const index = Number(input.value);
        if (Number.isNaN(index)) return;

        console.debug('[task-list] item click', {
          type: ev.type,
          index,
          checked: input.checked,
          value: input.value,
          item,
          input,
        });

        ev.stopPropagation();
        if ('stopImmediatePropagation' in ev) ev.stopImmediatePropagation();

        if (ev.target !== input) {
          console.debug('[task-list] forwarding click to checkbox', { index, input });
          input.click();
        }
      };

      itemHandlers.set(item, handler);
      item.addEventListener('click', handler);
      console.debug('[task-list] handler attached to item', item);
    }

    for (const [item, handler] of itemHandlers) {
      if (next.has(item)) continue;

      item.removeEventListener('click', handler);
      itemHandlers.delete(item);
      console.debug('[task-list] handler detached from item', item);
    }

    for (const [input, handler] of checkboxHandlers) {
      const stillPresent = items.some((item) => item.contains(input));
      if (stillPresent) continue;

      input.removeEventListener('change', handler);
      checkboxHandlers.delete(input);
      console.debug('[task-list] change handler detached from checkbox', input);
    }
  };

  const observer = new MutationObserver(() => syncHandlers());
  observer.observe(previewEl, { childList: true, subtree: true });

  syncHandlers();
  console.debug('[task-list] handler attached to element', previewEl);
  return () => {
    observer.disconnect();
    for (const [item, handler] of itemHandlers) {
      item.removeEventListener('click', handler);
    }
    for (const [input, handler] of checkboxHandlers) {
      input.removeEventListener('change', handler);
    }
    itemHandlers.clear();
    checkboxHandlers.clear();
    console.debug('[task-list] handler detached from element', previewEl);
  };
}
