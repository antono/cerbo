import { $, browser, expect } from '@wdio/globals';

// Spec `desktop-agent-panel`, "Empty and error states": without an agent the
// Agent tab says so and offers a check again instead of a chat. The harness
// (e2e/session.ts) gives spec files named `*no-agent*` an `agents.toml` in
// which neither OpenCode nor Claude Code can be found, whatever this machine
// has installed.

const byTestId = (id: string) => $(`[data-testid="${id}"]`);

describe('the Agent tab without an agent', () => {
  it('says no agent is installed and offers to check again', async () => {
    await byTestId('right-tab-agent').waitForDisplayed({ timeout: 30_000 });
    await byTestId('right-tab-agent').click();
    const panel = byTestId('agent-panel');
    await panel.waitForDisplayed({ timeout: 10_000 });
    await browser.waitUntil(async () => (await panel.getAttribute('data-view')) === 'no_agent', {
      timeout: 30_000,
      timeoutMsg: `the Agent tab never showed the empty state (view: ${await panel.getAttribute('data-view')})`,
    });
    const empty = byTestId('agent-empty-no-agent');
    await expect(empty).toBeExisting();
    // Both agents are listed, with the command to install.
    const text = (await empty.getProperty('textContent')) as string;
    expect(text).toContain('opencode');
    expect(text).toContain('claude-agent-acp');
    // No chat: no composer, no feed.
    expect(await byTestId('agent-composer').isExisting()).toBe(false);
    expect(await byTestId('agent-feed').isExisting()).toBe(false);
    // The connection itself is fine.
    await expect(byTestId('agent-connection')).toHaveAttribute('data-state', 'connected');
  });

  it('still has no agent after checking again', async () => {
    await byTestId('agent-recheck').click();
    await browser.pause(500);
    await expect(byTestId('agent-empty-no-agent')).toBeExisting();
  });
});
