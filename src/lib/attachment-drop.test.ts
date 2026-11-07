import { describe, expect, it } from 'vitest';
import {
  attachmentFilePath,
  attachmentReference,
  baseName,
  cerboObjectUuid,
  classifyDrop,
  classifyPaste,
  decodeDataUrl,
  draggedImageUrl,
  dropPositionToClient,
  fileUriToPath,
  fragmentName,
  isImageName,
  looksLikeCopiedPaths,
  nativeDropSource,
  parseGnomeCopiedFiles,
  parseUriList,
  rebaseRange,
  type TransferLike,
} from './attachment-drop';

const UUID = 'f7e435db-9740-4a2a-8e57-b439c4f8bb18';

/** A DataTransfer/clipboardData stand-in holding `data` and `files`. */
function transfer(data: Record<string, string>, files: File[] = []): TransferLike {
  const types = [...Object.keys(data), ...(files.length ? ['Files'] : [])];
  return { types, files, getData: (type) => data[type] ?? '' };
}

const png = () => new File([new Uint8Array([137])], 'photo.png', { type: 'image/png' });
const pdf = () => new File([new Uint8Array([137])], 'report.pdf', { type: 'application/pdf' });

describe('attachmentReference', () => {
  it('embeds images and links everything else', () => {
    expect(attachmentReference('photo.png', UUID)).toBe(`![photo.png](cerbo://objects/${UUID})`);
    expect(attachmentReference('Shot.JPEG', UUID)).toBe(`![Shot.JPEG](cerbo://objects/${UUID})`);
    expect(attachmentReference('report.pdf', UUID)).toBe(`[report.pdf](cerbo://objects/${UUID})`);
    expect(attachmentReference('notes', UUID)).toBe(`[notes](cerbo://objects/${UUID})`);
  });

  it('escapes brackets so the label stays one link', () => {
    expect(attachmentReference('a [137] b.txt', UUID)).toBe(`[a \\[137\\] b.txt](cerbo://objects/${UUID})`);
  });
});

describe('isImageName', () => {
  it('matches the extensions the core stores as image/*', () => {
    for (const name of ['a.png', 'a.jpg', 'a.jpeg', 'a.gif', 'a.webp', 'a.svg', 'a.BMP']) {
      expect(isImageName(name), name).toBe(true);
    }
    for (const name of ['a.pdf', 'a.html', 'png', '.png', 'a.png.txt']) {
      expect(isImageName(name), name).toBe(false);
    }
  });
});

describe('fragmentName', () => {
  const now = new Date(2026, 10, 7, 13, 7, 9);

  it('keeps a source name that matches its type', () => {
    expect(fragmentName('photo.png', 'image/png', 'pasted', now)).toBe('photo.png');
    expect(fragmentName('photo.jpeg', 'image/jpeg', 'pasted', now)).toBe('photo.jpeg');
    expect(fragmentName('report.pdf', 'application/pdf', 'pasted', now)).toBe('report.pdf');
  });

  it('adds the extension the type implies', () => {
    expect(fragmentName('image', 'image/png', 'pasted', now)).toBe('image.png');
    expect(fragmentName('page', 'text/html; charset=utf-8', 'dropped', now)).toBe('page.html');
  });

  it('names unnamed fragments after the time', () => {
    expect(fragmentName('', 'image/png', 'pasted', now)).toBe('pasted-20261107-130709.png');
    expect(fragmentName(undefined, 'text/plain', 'dropped', now)).toBe('dropped-20261107-130709.txt');
    expect(fragmentName(undefined, 'application/x-unknown', 'dropped', now)).toBe('dropped-20261107-130709');
  });

  it('drops directories from the name', () => {
    expect(fragmentName('../../x/photo.png', 'image/png', 'pasted', now)).toBe('photo.png');
    expect(fragmentName('..', 'image/png', 'pasted', now)).toBe('pasted-20261107-130709.png');
  });
});

describe('file URI parsing', () => {
  it('decodes local file URIs', () => {
    expect(fileUriToPath('file:///home/u/My%20Report%20137.pdf')).toBe('/home/u/My Report 137.pdf');
    expect(fileUriToPath('file://localhost/tmp/a.png')).toBe('/tmp/a.png');
    expect(fileUriToPath('https://example.org/a.png')).toBeNull();
    expect(fileUriToPath('file://otherhost/a.png')).toBeNull();
    expect(fileUriToPath('file:///bad%zz')).toBeNull();
  });

  it('reads text/uri-list, skipping comments and remote URIs', () => {
    const list = '# copied\r\nfile:///tmp/a.png\r\nhttps://example.org/b.png\r\nfile:///tmp/c%20d.pdf\r\n';
    expect(parseUriList(list)).toEqual(['/tmp/a.png', '/tmp/c d.pdf']);
  });

  it('reads x-special/gnome-copied-files', () => {
    expect(parseGnomeCopiedFiles('copy\nfile:///tmp/a.png\nfile:///tmp/b.pdf')).toEqual(['/tmp/a.png', '/tmp/b.pdf']);
    expect(parseGnomeCopiedFiles('cut\nfile:///tmp/a.png')).toEqual(['/tmp/a.png']);
    expect(parseGnomeCopiedFiles('file:///tmp/a.png')).toEqual(['/tmp/a.png']);
  });
});

describe('classifyDrop', () => {
  it('prefers files, in the order they were dropped', () => {
    const result = classifyDrop(transfer({ 'text/html': '<img>' }, [pdf(), png()]));
    expect(result.kind).toBe('files');
    if (result.kind === 'files') expect(result.files.map((f) => f.name)).toEqual(['report.pdf', 'photo.png']);
  });

  it('downloads an image dragged out of a web page without a file', () => {
    const html = '<meta charset="utf-8"><img src="https://example.org/cat.png?w=137" alt="cat">';
    expect(classifyDrop(transfer({ 'text/html': html, 'text/uri-list': 'https://example.org/cat.png?w=137' }))).toEqual({
      kind: 'urls',
      urls: ['https://example.org/cat.png?w=137'],
    });
  });

  it('prefers HTML over plain text', () => {
    expect(classifyDrop(transfer({ 'text/plain': 'hi', 'text/html': '<b>hi</b>' }))).toEqual({
      kind: 'html',
      html: '<b>hi</b>',
    });
  });

  it('stores a plain text selection as text', () => {
    expect(classifyDrop(transfer({ 'text/plain': 'one hundred thirty-seven' }))).toEqual({
      kind: 'text',
      text: 'one hundred thirty-seven',
    });
  });

  it('ignores empty drags', () => {
    expect(classifyDrop(transfer({ 'text/html': '  ' }))).toEqual({ kind: 'none' });
  });
});

describe('classifyPaste', () => {
  it('copies files from a file manager by path', () => {
    expect(
      classifyPaste(transfer({ 'x-special/gnome-copied-files': 'copy\nfile:///tmp/report.pdf', 'text/plain': '/tmp/report.pdf' })),
    ).toEqual({ kind: 'paths', paths: ['/tmp/report.pdf'] });
    expect(classifyPaste(transfer({ 'text/uri-list': 'file:///tmp/a.png' }, [png()]))).toEqual({
      kind: 'paths',
      paths: ['/tmp/a.png'],
    });
  });

  it('uses image data over the HTML and URL beside it', () => {
    const result = classifyPaste(
      transfer({ 'text/html': '<img src="https://example.org/a.png">', 'text/uri-list': 'https://example.org/a.png' }, [png()]),
    );
    expect(result.kind).toBe('files');
    if (result.kind === 'files') expect(result.files.map((f) => f.name)).toEqual(['photo.png']);
  });

  it('uploads other file data', () => {
    const result = classifyPaste(transfer({}, [pdf()]));
    expect(result.kind).toBe('files');
  });

  it('leaves text-only pastes to the default', () => {
    expect(classifyPaste(transfer({ 'text/plain': 'hello', 'text/html': '<p>hello</p>' }))).toEqual({ kind: 'none' });
  });
});

describe('paths and URLs', () => {
  it('builds the file path of an Attachment object', () => {
    expect(attachmentFilePath('/v/', UUID, 'photo.png')).toBe(`/v/.cerbo/objects/${UUID}/photo.png`);
    expect(attachmentFilePath('/v', UUID, 'meta.ttl')).toBe(`/v/.cerbo/objects/${UUID}/attachment-meta.ttl`);
  });

  it('reads the UUID of a cerbo URL', () => {
    expect(cerboObjectUuid(`cerbo://objects/${UUID}`)).toBe(UUID);
    expect(cerboObjectUuid(`cerbo://${UUID}`)).toBe(UUID);
    expect(cerboObjectUuid('assets/photo.png')).toBeNull();
  });

  it('takes the last path component', () => {
    expect(baseName('/tmp/x/report.pdf')).toBe('report.pdf');
    expect(baseName('C:\\x\\a.png')).toBe('a.png');
  });

});

describe('dropPositionToClient', () => {
  const linux = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)';
  const mac = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)';

  it('keeps Linux positions, which GTK already reports in CSS pixels', () => {
    // A drop on the right half of an 800 px wide editor at DPR 2 stays there.
    expect(dropPositionToClient({ x: 700, y: 137 }, 2, linux)).toEqual({ x: 700, y: 137 });
  });

  it('divides physical positions elsewhere', () => {
    expect(dropPositionToClient({ x: 274, y: 137 }, 2, mac)).toEqual({ x: 137, y: 68.5 });
    expect(dropPositionToClient({ x: 137, y: 137 }, 0, mac)).toEqual({ x: 137, y: 137 });
  });
});

describe('draggedImageUrl', () => {
  it('reads a single image, also inside a link', () => {
    expect(draggedImageUrl(transfer({ 'text/html': '<a href="/x"><img src=\'https://e.org/a.jpg\'></a>' }))).toBe(
      'https://e.org/a.jpg',
    );
    expect(draggedImageUrl(transfer({ 'text/html': '<img src="https://e.org/a?x=1&amp;y=2">' }))).toBe('https://e.org/a?x=1&y=2');
    expect(draggedImageUrl(transfer({ 'text/html': '<img src="data:image/png;base64,iVBO">' }))).toBe('data:image/png;base64,iVBO');
  });

  it('ignores HTML with more than an image', () => {
    expect(draggedImageUrl(transfer({ 'text/html': '<p>137</p><img src="https://e.org/a.png">' }))).toBeNull();
    expect(draggedImageUrl(transfer({ 'text/html': '<img src="/relative.png">' }))).toBeNull();
  });

  it('falls back to an image URL in text/uri-list', () => {
    expect(draggedImageUrl(transfer({ 'text/uri-list': '# x\nhttps://e.org/a.webp?s=1' }))).toBe('https://e.org/a.webp?s=1');
    expect(draggedImageUrl(transfer({ 'text/uri-list': 'https://e.org/page' }))).toBeNull();
    expect(draggedImageUrl(transfer({ 'text/uri-list': 'file:///tmp/a.png' }))).toBeNull();
  });
});

describe('looksLikeCopiedPaths', () => {
  it('matches the path lists file managers put beside copied files', () => {
    expect(looksLikeCopiedPaths('/home/u/report.pdf')).toBe(true);
    expect(looksLikeCopiedPaths('file:///tmp/a.png\nfile:///tmp/b%20137.pdf\n')).toBe(true);
    expect(looksLikeCopiedPaths('see /home/u/report.pdf')).toBe(false);
    expect(looksLikeCopiedPaths('https://example.org/a.png')).toBe(false);
    expect(looksLikeCopiedPaths('  ')).toBe(false);
  });
});

describe('nativeDropSource', () => {
  it('copies local paths, downloads web URLs and rejects the rest', () => {
    expect(nativeDropSource('/home/u/report.pdf')).toBe('path');
    expect(nativeDropSource('C:\\Users\\u\\a.png')).toBe('path');
    expect(nativeDropSource('https://example.org/a.png')).toBe('url');
    expect(nativeDropSource('http://example.org/')).toBe('url');
    expect(nativeDropSource('ftp://example.org/a.png')).toBe('unsupported');
    expect(nativeDropSource('relative/a.png')).toBe('unsupported');
  });
});

describe('decodeDataUrl', () => {
  it('decodes base64 and percent-encoded data', () => {
    expect(decodeDataUrl('data:image/png;base64,iQ==')).toEqual({ mime: 'image/png', bytes: new Uint8Array([137]) });
    expect(decodeDataUrl('data:image/svg+xml,%3Csvg%2F%3E')).toEqual({
      mime: 'image/svg+xml',
      bytes: new TextEncoder().encode('<svg/>'),
    });
    expect(decodeDataUrl('data:image/png;base64,***')).toBeNull();
    expect(decodeDataUrl('https://e.org/a.png')).toBeNull();
  });
});

describe('rebaseRange', () => {
  const range = { start: 6, end: 6 };

  it('keeps the range when nothing changed or the edit came after it', () => {
    expect(rebaseRange('hello world', 'hello world', range)).toEqual(range);
    expect(rebaseRange('hello world', 'hello world 137', range)).toEqual(range);
    // Typing on at the drop point leaves the reference before the new text.
    expect(rebaseRange('hello world', 'hello 137 world', range)).toEqual(range);
  });

  it('shifts the range past edits before it', () => {
    expect(rebaseRange('hello world', '137 hello world', range)).toEqual({ start: 10, end: 10 });
    expect(rebaseRange('hello world', 'hlo world', range)).toEqual({ start: 4, end: 4 });
  });

  it('gives up when the edit touches the range', () => {
    expect(rebaseRange('hello world', 'hello', { start: 6, end: 11 })).toBeNull();
    expect(rebaseRange('hello world', 'help world', { start: 4, end: 4 })).toBeNull();
  });
});
