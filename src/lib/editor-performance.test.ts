import { describe, expect, it, vi } from 'vitest';
import { Carta } from 'carta-md';
import {
  LARGE_DOCUMENT_CHARS,
  LARGE_DOCUMENT_EXIT_CHARS,
  captureInputView,
  gatePreviewRendering,
  inputScroller,
  nextPlainInput,
  offsetCaretByTextareaScroll,
  restoreInputView,
  usePlainInput,
  type CaretTarget,
} from './editor-performance';

function fakeCarta() {
  return {
    render: vi.fn(async (md: string) => `<p>${md}</p>`),
    renderSSR: vi.fn((md: string) => `<p>${md}</p>`),
  };
}

describe('usePlainInput', () => {
  it('keeps the overlay below the threshold', () => {
    expect(usePlainInput(0)).toBe(false);
    expect(usePlainInput(137)).toBe(false);
    expect(usePlainInput(LARGE_DOCUMENT_CHARS - 1)).toBe(false);
  });

  it('switches to the plain textarea at and above the threshold', () => {
    expect(usePlainInput(LARGE_DOCUMENT_CHARS)).toBe(true);
    expect(usePlainInput(137 * 1024)).toBe(true);
  });
});

describe('gatePreviewRendering', () => {
  it('skips both render paths while the preview is hidden', async () => {
    const carta = fakeCarta();
    const { render, renderSSR } = carta;
    gatePreviewRendering(carta, () => false);

    expect(carta.renderSSR('# 137')).toBe('');
    expect(await carta.render('# 137')).toBe('');
    expect(render).not.toHaveBeenCalled();
    expect(renderSSR).not.toHaveBeenCalled();
  });

  it('renders when the preview is visible, reading the flag per call', async () => {
    const carta = fakeCarta();
    let visible = false;
    gatePreviewRendering(carta, () => visible);

    expect(carta.renderSSR('a')).toBe('');
    visible = true;
    expect(carta.renderSSR('a')).toBe('<p>a</p>');
    expect(await carta.render('b')).toBe('<p>b</p>');
  });

  it('restores the original methods', () => {
    const carta = fakeCarta();
    const { render, renderSSR } = carta;
    const restore = gatePreviewRendering(carta, () => false);
    restore();
    expect(carta.render).toBe(render);
    expect(carta.renderSSR).toBe(renderSSR);
  });

  it('keeps `this` bound on a real Carta instance', async () => {
    const carta = new Carta({ sanitizer: false });
    let visible = false;
    gatePreviewRendering(carta, () => visible);

    expect(carta.renderSSR('**137**')).toBe('');
    visible = true;
    expect(carta.renderSSR('**137**')).toContain('<strong>137</strong>');
  });
});

describe('nextPlainInput', () => {
  it('switches a growing page to the plain textarea at the threshold', () => {
    expect(nextPlainInput(false, LARGE_DOCUMENT_CHARS - 1)).toBe(false);
    expect(nextPlainInput(false, LARGE_DOCUMENT_CHARS)).toBe(true);
  });

  it('keeps a shrinking page plain until it is well below the threshold', () => {
    expect(nextPlainInput(true, LARGE_DOCUMENT_CHARS - 137)).toBe(true);
    expect(nextPlainInput(true, LARGE_DOCUMENT_EXIT_CHARS)).toBe(true);
    expect(nextPlainInput(true, LARGE_DOCUMENT_EXIT_CHARS - 1)).toBe(false);
  });
});

// Minimal stand-ins: vitest runs without a DOM here.
function fakeTextarea(plain: boolean, container: { scrollTop: number } | null) {
  const doc = { activeElement: null as unknown };
  const textarea = {
    selectionStart: 0,
    selectionEnd: 0,
    scrollTop: 0,
    scrollLeft: 0,
    ownerDocument: doc,
    classList: { contains: (c: string) => plain && c === 'carta-plain-input' },
    closest: () => container,
    focus: vi.fn(() => (doc.activeElement = textarea)),
    setSelectionRange(start: number, end: number) {
      this.selectionStart = start;
      this.selectionEnd = end;
    },
  };
  return textarea as typeof textarea & HTMLTextAreaElement;
}

describe('input view across an editor swap', () => {
  it('scrolls the plain textarea itself and the overlay input through its container', () => {
    const container = { scrollTop: 0 };
    expect(inputScroller(fakeTextarea(true, container))).not.toBe(container);
    expect(inputScroller(fakeTextarea(false, container))).toBe(container);
  });

  it('carries caret, selection, scroll and focus from the overlay input to the plain textarea', () => {
    const container = { scrollTop: 1370 };
    const overlay = fakeTextarea(false, container);
    overlay.setSelectionRange(137, 140);
    overlay.ownerDocument.activeElement = overlay;
    const view = captureInputView(overlay);
    expect(view).toEqual({ selectionStart: 137, selectionEnd: 140, scrollTop: 1370, focused: true });

    const plain = fakeTextarea(true, null);
    restoreInputView(plain, view);
    expect(plain.focus).toHaveBeenCalledWith({ preventScroll: true });
    expect([plain.selectionStart, plain.selectionEnd]).toEqual([137, 140]);
    expect(plain.scrollTop).toBe(1370);
  });

  it('does not focus a textarea that was not focused', () => {
    const plain = fakeTextarea(true, null);
    restoreInputView(plain, { selectionStart: 1, selectionEnd: 1, scrollTop: 0, focused: false });
    expect(plain.focus).not.toHaveBeenCalled();
  });
});

describe('offsetCaretByTextareaScroll', () => {
  it('moves caret positions up and left by the textarea scroll offset', () => {
    const textarea = fakeTextarea(true, null);
    textarea.selectionStart = 137;
    const getCursorXY = vi.fn((at?: number) => ({ x: 10, y: 5000 + (at ?? 0), left: 10, top: 5000 + (at ?? 0), right: 0, bottom: 0 }));
    const input: CaretTarget = { textarea, getCursorXY };
    offsetCaretByTextareaScroll(input);

    textarea.scrollTop = 4800;
    textarea.scrollLeft = 4;
    const xy = input.getCursorXY();
    expect(getCursorXY).toHaveBeenCalledWith(137);
    expect(xy).toMatchObject({ x: 6, left: 6, y: 337, top: 337 });

    textarea.scrollTop = 0;
    textarea.scrollLeft = 0;
    expect(input.getCursorXY(0)).toMatchObject({ y: 5000, top: 5000 });
  });
});
