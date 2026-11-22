import { describe, expect, it } from 'vitest';
import { ACTION_LABELS, hostSummary, modeLabel, NO_HOST_ACCESS, parseHostAccess, writeWarning } from './host';

describe('parseHostAccess', () => {
  it('reads a setting from the registry', () => {
    expect(parseHostAccess({ enabled: true, paths: [{ path: '/data/in', write: false }, { path: '/data/out', write: true }] })).toEqual({
      enabled: true,
      paths: [
        { path: '/data/in', write: false },
        { path: '/data/out', write: true },
      ],
    });
  });

  it('reads anything unreadable as off with no paths', () => {
    for (const bad of [null, undefined, 'on', 7, [], {}, { enabled: 'yes' }, { enabled: 1, paths: 'x' }]) {
      expect(parseHostAccess(bad)).toEqual(NO_HOST_ACCESS);
    }
  });

  it('never turns a missing or odd mode into write access', () => {
    const host = parseHostAccess({ enabled: true, paths: [{ path: '/a' }, { path: '/b', write: 'true' }, { path: 3 }, null] });
    expect(host.paths).toEqual([
      { path: '/a', write: false },
      { path: '/b', write: false },
    ]);
  });
});

describe('wording', () => {
  it('names the mode', () => {
    expect(modeLabel({ path: '/a', write: false })).toBe('read only');
    expect(modeLabel({ path: '/a', write: true })).toBe('read and write');
  });

  it('says nothing when host access is off and counts the list when on', () => {
    expect(hostSummary(NO_HOST_ACCESS)).toBeNull();
    expect(hostSummary({ enabled: true, paths: [] })).toMatch(/every path asks/);
    expect(hostSummary({ enabled: true, paths: [{ path: '/a', write: false }] })).toBe('Host access on: 1 allowed path, others ask');
    expect(hostSummary({ enabled: true, paths: [{ path: '/a', write: false }, { path: '/b', write: true }] })).toMatch(/2 allowed paths/);
  });

  it('warns about writing with the path', () => {
    expect(writeWarning('/home/me/out')).toContain('/home/me/out');
  });

  it('labels every action the daemon can ask about besides page changes', () => {
    for (const action of ['host list', 'host read', 'host write', 'ontology', 'add_type', 'import_ontology', 'reindex', 'restore']) {
      expect(ACTION_LABELS[action]).toBeTruthy();
    }
  });
});
