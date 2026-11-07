import { describe, expect, it, vi } from 'vitest';
import { unified, type Processor } from 'unified';
import remarkParse from 'remark-parse';
import remarkRehype from 'remark-rehype';
import rehypeStringify from 'rehype-stringify';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
}));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('./WikilinkAutocomplete.svelte', () => ({ default: {} }));

const { wikilinkPlugin } = await import('./wikilink-plugin');

const UUID = 'f7e435db-9740-4a2a-8e57-b439c4f8bb18';

/** Render `markdown` through the plugin's remark and rehype transformers, as Carta does. */
async function render(markdown: string, objects: { uuid: string; title: string; objectType: string }[]) {
  const plugin = wikilinkPlugin({
    getPages: () => [],
    getVaultObjects: () => objects,
    onNavigate: () => {},
    onCreate: () => {},
    getVaultPath: () => '/vault',
    getSlug: () => 'page',
  });
  const run = (type: 'remark' | 'rehype', processor: Processor<any, any, any, any, any>) => {
    for (const t of plugin.transformers ?? []) {
      if (t.type === type) (t.transform as (args: { processor: unknown }) => void)({ processor });
    }
  };
  const processor = unified().use(remarkParse) as unknown as Processor<any, any, any, any, any>;
  run('remark', processor);
  processor.use(remarkRehype);
  run('rehype', processor);
  processor.use(rehypeStringify);
  return String(await processor.process(markdown));
}

describe('wikilinkPlugin cerbo:// images', () => {
  it('shows an Attachment object image from the vault', async () => {
    const html = await render(`![photo.png](cerbo://objects/${UUID})`, [
      { uuid: UUID, title: 'photo.png', objectType: 'Attachment' },
    ]);
    const path = encodeURIComponent(`/vault/.cerbo/objects/${UUID}/photo.png`);
    expect(html).toContain(`src="asset://localhost/${path}"`);
    expect(html).toContain(`data-cerbo-uuid="${UUID}"`);
  });

  it('falls back to the alt text before the object list has the new object', async () => {
    const html = await render(`![shot.png](cerbo://objects/${UUID})`, []);
    expect(html).toContain(encodeURIComponent(`/vault/.cerbo/objects/${UUID}/shot.png`));
  });

  it('leaves other images alone', async () => {
    const html = await render('![x](https://example.org/137.png)', []);
    expect(html).toContain('src="https://example.org/137.png"');
  });
});
