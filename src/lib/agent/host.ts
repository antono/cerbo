/**
 * Host access of a vault (change `add-agent-host-scope-and-mcp-skill`, spec
 * `agent-host-access`): letting the agent work with files outside the vault.
 * A path on the list is open within its mode; any other path asks per action.
 */

export interface HostPath {
  path: string;
  /** `write` includes `read`. */
  write: boolean;
}

export interface HostAccess {
  enabled: boolean;
  paths: HostPath[];
}

export const NO_HOST_ACCESS: HostAccess = { enabled: false, paths: [] };

/** A received value as host access; anything unreadable is off with no paths, so it never widens access. */
export function parseHostAccess(value: unknown): HostAccess {
  if (!value || typeof value !== 'object') return NO_HOST_ACCESS;
  const raw = value as { enabled?: unknown; paths?: unknown };
  const paths = Array.isArray(raw.paths)
    ? raw.paths
        .filter((p): p is { path: string; write?: unknown } => !!p && typeof p === 'object' && typeof (p as { path?: unknown }).path === 'string')
        .map((p) => ({ path: p.path, write: p.write === true }))
    : [];
  return { enabled: raw.enabled === true, paths };
}

/** The mode of a path in words. */
export function modeLabel(path: HostPath): string {
  return path.write ? 'read and write' : 'read only';
}

/** The confirmation text for turning host access on. */
export const HOST_ON_WARNING =
  'The agent will be able to read, and where you allow it write, files outside this vault. Paths on your list open without asking; any other path asks you for each action. Your credentials (~/.ssh, ~/.gnupg, ~/.aws) and Cerbo’s own files stay closed. This reduces isolation: keep the list short.';

/** The confirmation text for allowing writing to a path. */
export function writeWarning(path: string): string {
  return `The agent will be able to create and replace files under ${path} without asking.`;
}

/** A one-line summary for the panel: nothing when off. */
export function hostSummary(host: HostAccess): string | null {
  if (!host.enabled) return null;
  const n = host.paths.length;
  return n === 0 ? 'Host access on: every path asks' : `Host access on: ${n} allowed ${n === 1 ? 'path' : 'paths'}, others ask`;
}

/** What an approval card says for the actions that are not page changes. */
export const ACTION_LABELS: Record<string, string> = {
  'host list': 'List a folder outside the vault',
  'host read': 'Read a file outside the vault',
  'host write': 'Write a file outside the vault',
  ontology: 'Create a vocabulary',
  add_type: 'Add a type to a vocabulary',
  import_ontology: 'Import an ontology',
  reindex: 'Rebuild the index',
  restore: 'Restore a page from the trash',
};
