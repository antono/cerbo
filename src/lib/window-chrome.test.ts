import { describe, expect, it, vi } from 'vitest';
import { QUIT_PROMPT, requestQuit, titleParts } from './window-chrome';

describe('window-chrome', () => {
  it('close button opens the exit prompt after closing other dialogs', () => {
    const state = { showExitPrompt: false, showSearch: true };
    const closeDialogs = vi.fn(() => {
      state.showSearch = false;
      state.showExitPrompt = false;
    });

    requestQuit(state, closeDialogs);

    expect(closeDialogs).toHaveBeenCalledTimes(1);
    expect(state.showSearch).toBe(false);
    expect(state.showExitPrompt).toBe(true);
  });

  it('asks the question the application-lifecycle spec names', () => {
    expect(QUIT_PROMPT.message).toBe('Are you sure you want to quit?');
  });

  it('builds the vault and page title parts', () => {
    expect(titleParts('Notes', 'Index')).toEqual(['Notes', 'Index']);
    expect(titleParts('Notes', null)).toEqual(['Notes']);
    expect(titleParts(undefined, '  ')).toEqual([]);
    expect(titleParts('Vault 137', 'Page 137')).toEqual(['Vault 137', 'Page 137']);
  });
});
