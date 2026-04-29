import { describe, expect, it, vi } from 'vitest';
import { focusTextareaAtOffset } from './textarea-focus';

describe('textarea-focus', () => {
  it('focuses without scrolling the window', () => {
    const focus = vi.fn();
    const setSelectionRange = vi.fn();

    focusTextareaAtOffset({ focus, setSelectionRange }, 17);

    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    expect(setSelectionRange).toHaveBeenCalledWith(17, 17);
  });

  it('falls back when focus options are unsupported', () => {
    const focus = vi.fn()
      .mockImplementationOnce(() => {
        throw new Error('unsupported');
      })
      .mockImplementationOnce(() => undefined);
    const setSelectionRange = vi.fn();

    focusTextareaAtOffset({ focus, setSelectionRange }, 4);

    expect(focus).toHaveBeenNthCalledWith(1, { preventScroll: true });
    expect(focus).toHaveBeenNthCalledWith(2);
    expect(setSelectionRange).toHaveBeenCalledWith(4, 4);
  });
});
