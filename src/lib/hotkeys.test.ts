import { describe, expect, it } from 'vitest';
import { confirmationOpen, isModKey, isSparqlConsoleKey } from './hotkeys';

// Under vitest there is no browser, so the primary modifier is Ctrl.
function key(init: Partial<KeyboardEvent>): KeyboardEvent {
  return { key: '', ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...init } as KeyboardEvent;
}

describe('hotkeys', () => {
  it('matches the primary modifier case-insensitively', () => {
    expect(isModKey(key({ key: 'P', ctrlKey: true }), 'p')).toBe(true);
    expect(isModKey(key({ key: 'p' }), 'p')).toBe(false);
  });

  it('opens the SPARQL console with Ctrl+Shift+E only', () => {
    // With Shift held the browser reports an upper-case key.
    expect(isSparqlConsoleKey(key({ key: 'E', ctrlKey: true, shiftKey: true }))).toBe(true);
    expect(isSparqlConsoleKey(key({ key: 'e', ctrlKey: true }))).toBe(false);
    expect(isSparqlConsoleKey(key({ key: 'E', shiftKey: true }))).toBe(false);
    expect(isSparqlConsoleKey(key({ key: 'E', ctrlKey: true, shiftKey: true, altKey: true }))).toBe(false);
  });

  it('reports an open confirmation modal', () => {
    expect(confirmationOpen({ showExitPrompt: false, confirmDeleteUuid: null })).toBe(false);
    expect(confirmationOpen({ showExitPrompt: true, confirmDeleteUuid: null })).toBe(true);
    expect(confirmationOpen({ showExitPrompt: false, confirmDeleteUuid: 'page-137' })).toBe(true);
  });
});
