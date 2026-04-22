/**
 * Hotkey utility for cross-platform keyboard event normalization.
 * Maps Ctrl (Linux/Windows) to Meta/Cmd (Mac).
 */

export const isMac = typeof window !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent);

/**
 * Checks if the given keyboard event matches the primary modifier and key.
 * @param e - KeyboardEvent
 * @param key - The key name (case-insensitive)
 */
export function isModKey(e: KeyboardEvent, key: string): boolean {
  const mod = isMac ? e.metaKey : e.ctrlKey;
  return mod && e.key.toLowerCase() === key.toLowerCase();
}

/**
 * Checks if the event target is an input-like element (input, textarea, select, contenteditable).
 */
export function isInputFocused(): boolean {
  const active = document.activeElement;
  if (!active) return false;
  
  const tag = active.tagName.toLowerCase();
  return (
    tag === 'input' || 
    tag === 'textarea' || 
    tag === 'select' || 
    (active as HTMLElement).isContentEditable
  );
}
