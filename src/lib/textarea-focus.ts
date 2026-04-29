interface TextareaLike {
  focus(options?: FocusOptions): void;
  setSelectionRange(start: number, end: number): void;
}

export function focusTextareaAtOffset(textarea: TextareaLike, offset: number) {
  try {
    textarea.focus({ preventScroll: true });
  } catch {
    textarea.focus();
  }
  textarea.setSelectionRange(offset, offset);
}
