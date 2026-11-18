import { $, browser, expect } from '@wdio/globals';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { sessionSandboxed, sessionVaultDir, writeAgentScript } from '../../session.ts';

// Spec `desktop-agent-panel` and `agent-sessions` (change `add-agent-panel`,
// task 5.1): the Agent tab against the real daemon, which the harness
// (e2e/session.ts) starts per session with a scripted fake ACP agent standing
// in for OpenCode. The daemon runs the agent in the real bubblewrap sandbox
// where this machine can (`CERBO_BWRAP`), else without one.
//
// The fake agent reads its script once, when a session starts, so every test
// writes its script first and then starts a new session.
//
// WebKitWebDriver reports no rendered text, so the assertions read the DOM.

const PAGE = 'Index';

const byTestId = (id: string) => $(`[data-testid="${id}"]`);

/** Texts of all elements of `selector`, read from the DOM. */
function texts(selector: string): Promise<string[]> {
  return browser.execute(
    (sel) => Array.from(document.querySelectorAll(sel)).map((el) => (el.textContent ?? '').trim()),
    selector,
  );
}

const agentMessages = () => texts('[data-testid="agent-message"][data-role="agent"]');
// The user's message also lists the context pages (the open page); only `.text` is what was typed.
const userMessages = () => texts('[data-testid="agent-message"][data-role="user"] .text');

async function waitForAgentText(fragment: string, timeout = 20_000) {
  await browser.waitUntil(async () => (await agentMessages()).some((t) => t.includes(fragment)), {
    timeout,
    timeoutMsg: `the agent never said "${fragment}"; it said ${JSON.stringify(await agentMessages())}`,
  });
}

async function openAgentTab() {
  await byTestId('right-tab-agent').waitForDisplayed({ timeout: 30_000 });
  await byTestId('right-tab-agent').click();
  const panel = byTestId('agent-panel');
  await panel.waitForDisplayed({ timeout: 10_000 });
  await browser.waitUntil(async () => (await panel.getAttribute('data-view')) === 'ready', {
    timeout: 30_000,
    timeoutMsg: `the Agent tab never became ready (view: ${await panel.getAttribute('data-view')})`,
  });
}

async function permission(): Promise<string | null> {
  return byTestId('agent-permission').getAttribute('data-value');
}

/** Set the permission through the control, confirming a widening switch. */
async function setPermission(level: 'read' | 'approve' | 'auto') {
  if ((await permission()) === level) return;
  await byTestId(`agent-permission-${level}`).click();
  const confirm = byTestId('agent-permission-confirm');
  if (await confirm.isExisting()) await confirm.$('.btn-danger').click();
  await browser.waitUntil(async () => (await permission()) === level, {
    timeout: 10_000,
    timeoutMsg: `the permission did not become ${level}`,
  });
}

/** Start a fresh session (which ends the previous one) and send `text`. */
async function startAndSend(text: string) {
  await byTestId('agent-new-session').click();
  await browser.waitUntil(async () => (await texts('[data-testid="agent-feed-empty"]')).length === 1, {
    timeout: 20_000,
    timeoutMsg: 'the new session did not open with an empty feed',
  });
  await send(text);
}

async function send(text: string) {
  const input = byTestId('agent-input');
  await input.click();
  await input.setValue(text);
  await byTestId('agent-send').click();
}

/** The uuid of the tree row titled `title`. */
async function pageUuid(title: string): Promise<string> {
  const uuid = await browser.execute(
    (t) =>
      Array.from(document.querySelectorAll('.page-list .item.page'))
        .find((li) => li.querySelector('.label')?.textContent === t)
        ?.getAttribute('data-uuid'),
    title,
  );
  if (!uuid) throw new Error(`no page row "${title}"`);
  return uuid;
}

async function openRow(title: string) {
  const uuid = await pageUuid(title);
  await $(`.page-list .item.page[data-uuid="${uuid}"] .page-btn`).click();
  await waitForEditorText((t) => t.startsWith(`# ${title}`), `the editor to show "${title}"`);
  return uuid;
}

function editorText(): Promise<string> {
  return browser.execute(() => (document.querySelector('.carta-input textarea') as HTMLTextAreaElement | null)?.value ?? '');
}

async function waitForEditorText(test: (text: string) => boolean, what: string, timeout = 15_000) {
  await browser.waitUntil(async () => test(await editorText()), {
    timeout,
    timeoutMsg: `timed out waiting for ${what}; the editor holds ${JSON.stringify(await editorText())}`,
  });
}

/** The page on disk: the file the vault stores under its uuid. */
function pageOnDisk(uuid: string): string {
  return readFileSync(join(sessionVaultDir(), '.cerbo', 'objects', uuid, 'page.md'), 'utf8');
}

async function waitForApprovalCard() {
  const card = byTestId('agent-approval-card');
  await card.waitForDisplayed({ timeout: 20_000 });
  return card;
}

const sessionItems = () =>
  browser.execute(() =>
    Array.from(document.querySelectorAll('[data-testid="agent-session-item"]')).map((el) => ({
      id: el.getAttribute('data-session'),
      state: el.getAttribute('data-state'),
    })),
  );

async function openSessionsList() {
  if (!(await byTestId('agent-sessions').isExisting())) await byTestId('agent-sessions-toggle').click();
  await byTestId('agent-sessions').waitForDisplayed({ timeout: 5_000 });
}

describe('the Agent tab with a scripted agent', () => {
  let uuid: string;
  let original: string;

  before(async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    uuid = await openRow(PAGE);
    original = pageOnDisk(uuid);
    await openAgentTab();
  });

  it('is connected, shows the picked agent and starts read only', async () => {
    await expect(byTestId('agent-connection')).toHaveAttribute('data-state', 'connected');
    expect(await byTestId('agent-picker').getValue()).toBe('opencode');
    expect(await permission()).toBe('read');
    await expect(byTestId('agent-feed-empty')).toBeExisting();
  });

  it('starts a session on the first message and streams the reply with a tool row', async () => {
    writeAgentScript({
      turns: [
        [
          { do: 'tool_call', id: 't-1', title: 'Look at the page', kind: 'read', status: 'in_progress' },
          { do: 'tool_update', id: 't-1', status: 'completed', text: 'looked' },
          { do: 'text', text: 'Hello from the scripted agent 137', chunk_size: 6 },
        ],
      ],
    });
    await send('Say hello');
    await browser.waitUntil(async () => (await userMessages()).includes('Say hello'), {
      timeout: 20_000,
      timeoutMsg: 'the message did not show in the feed',
    });
    await waitForAgentText('Hello from the scripted agent 137');
    const row = byTestId('agent-tool-row');
    await row.waitForExist({ timeout: 10_000 });
    await expect(row).toHaveAttribute('data-status', 'done');
    expect(await row.getProperty('textContent')).toContain('Look at the page');
    // The composer is free again once the turn ended.
    await byTestId('agent-send').waitForExist({ timeout: 10_000 });
    // The panel warns exactly when the agent runs without the sandbox; the
    // harness uses real bubblewrap whenever this machine has it.
    expect(await byTestId('agent-sandbox-warning').isExisting()).toBe(!sessionSandboxed());
    await openSessionsList();
    expect((await sessionItems()).length).toBe(1);
    await byTestId('agent-sessions-toggle').click();
  });

  it('asks before widening the permission, and not before narrowing it', async () => {
    await byTestId('agent-permission-approve').click();
    const confirm = byTestId('agent-permission-confirm');
    await confirm.waitForExist({ timeout: 5_000 });
    // Nothing changed yet; Cancel leaves it as it was.
    expect(await permission()).toBe('read');
    await confirm.$('.btn-ghost').click();
    await confirm.waitForExist({ reverse: true, timeout: 5_000 });
    expect(await permission()).toBe('read');

    await setPermission('approve');
    expect(await permission()).toBe('approve');
    await setPermission('auto');
    // Narrowing is immediate: no dialog.
    await byTestId('agent-permission-approve').click();
    await browser.waitUntil(async () => (await permission()) === 'approve', { timeout: 5_000 });
    expect(await byTestId('agent-permission-confirm').isExisting()).toBe(false);
  });

  it('shows an approval card, applies the change on Approve and refreshes the open page', async () => {
    await setPermission('approve');
    const edited = '# Index\n\nEdited by the agent 137\n';
    writeAgentScript({
      turns: [
        [
          { do: 'text', text: 'Editing the page' },
          { do: 'mcp_call', tool: 'edit_page', arguments: { page: uuid, content: edited } },
          { do: 'text', text: 'The page is edited' },
        ],
      ],
    });
    await startAndSend('Edit the page');
    const card = await waitForApprovalCard();
    await expect(card).toHaveAttribute('data-outcome', 'pending');
    // Nothing is written while the card waits, on disk or in the editor.
    expect(pageOnDisk(uuid)).toBe(original);
    expect(await editorText()).toBe(original);
    expect(await card.$('[data-testid="agent-approval-diff"]').getProperty('textContent')).toContain('Edited by the agent 137');

    await card.$('[data-testid="agent-approve"]').click();
    await waitForAgentText('The page is edited');
    await browser.waitUntil(async () => (await byTestId('agent-approval-card').getAttribute('data-outcome')) === 'approved', {
      timeout: 10_000,
    });
    expect(pageOnDisk(uuid)).toBe(edited);
    // The open page shows the new text without being reopened.
    await waitForEditorText((t) => t === edited, 'the open page to refresh');
    await expect(byTestId('agent-page-link')).toBeExisting();
  });

  it('leaves the page alone on Reject and tells the agent', async () => {
    const before = pageOnDisk(uuid);
    writeAgentScript({
      turns: [
        [
          { do: 'text', text: 'Trying another edit' },
          { do: 'mcp_call', tool: 'edit_page', arguments: { page: uuid, content: '# Index\n\nrejected text 137\n' } },
          { do: 'text', text: 'Moving on after the answer' },
        ],
      ],
    });
    await startAndSend('Edit it again');
    const card = await waitForApprovalCard();
    await card.$('[data-testid="agent-reject"]').click();
    await browser.waitUntil(async () => (await byTestId('agent-approval-card').getAttribute('data-outcome')) === 'rejected', {
      timeout: 10_000,
    });
    await waitForAgentText('Moving on after the answer');
    expect(pageOnDisk(uuid)).toBe(before);
    expect(await editorText()).toBe(before);
    // The agent heard the refusal.
    expect((await agentMessages()).join('\n')).toContain('mcp_result:');
  });

  it('keeps the session across closing and reopening the app, with the messages made meanwhile', async () => {
    await setPermission('read');
    writeAgentScript({
      turns: [
        [{ do: 'text', text: 'First answer 137' }],
        [
          { do: 'text', text: 'Working on it' },
          { do: 'sleep', ms: 15_000 },
          { do: 'text', text: 'Late news while you were away 137' },
        ],
      ],
    });
    await startAndSend('First question');
    await waitForAgentText('First answer 137');
    await send('Second question');
    await waitForAgentText('Working on it');

    await openSessionsList();
    const before = await sessionItems();
    const live = before.find((s) => s.state === 'working');
    if (!live) throw new Error(`no working session in ${JSON.stringify(before)}`);

    // Close the app (the daemon runs on) and open it again on the same dirs.
    await browser.reloadSession();
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    await openAgentTab();

    // The same session is shown again, with what was said before...
    await browser.waitUntil(async () => (await userMessages()).includes('Second question'), {
      timeout: 20_000,
      timeoutMsg: `the old messages did not come back: ${JSON.stringify(await userMessages())}`,
    });
    expect(await userMessages()).toContain('First question');
    expect(await agentMessages()).toContain('First answer 137');
    await openSessionsList();
    const after = await sessionItems();
    expect(after.some((s) => s.id === live.id)).toBe(true);
    // ...and with what the agent said while no window was open.
    await waitForAgentText('Late news while you were away 137', 40_000);
  });

  it('lists the sessions of the vault', async () => {
    await openSessionsList();
    const items = await sessionItems();
    // The first message, then one per `startAndSend`.
    expect(items.length).toBeGreaterThanOrEqual(4);
    expect(new Set(items.map((s) => s.id)).size).toBe(items.length);
  });
});
