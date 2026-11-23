/** Pure helpers for the "Import vault" dialog (spec `vault-import-ui`). */

export type SourceFormat = 'obsidian' | 'tolaria';

/** Source apps in the order the dialog lists them: Obsidian first. */
export const SOURCE_FORMATS: { id: SourceFormat; label: string; hint: string }[] = [
  { id: 'obsidian', label: 'Obsidian', hint: 'A folder with .obsidian/' },
  { id: 'tolaria', label: 'Tolaria', hint: 'A folder of notes with type: frontmatter' },
];

export interface Detection {
  format: SourceFormat;
  reason: string;
}

export interface ImportReport {
  vault: { id: string | null; name: string | null; path: string } | null;
  source: string;
  format: SourceFormat;
  formatReason: string;
  dryRun: boolean;
  pages: number;
  attachments: number;
  attachmentBytes: number;
  relations: number;
  links: number;
  skipped: string[];
  unreferenced: string[];
  unresolved: { note: string; link: string }[];
  warnings: string[];
}

/** The format the dialog should offer instead of the chosen one, if any. */
export function formatMismatch(chosen: SourceFormat, detected: Detection | null): SourceFormat | null {
  return detected && detected.format !== chosen ? detected.format : null;
}

/** Default name of a new vault: the source folder's own name. */
export function defaultVaultName(source: string): string {
  const parts = source.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || 'Imported vault';
}

/** `1536` → `1.5 KB`. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}

/** `3 pages`, `1 page`. */
export function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

/** Event `vault-import-progress` from `vault_import_run`. */
export interface ImportProgress {
  phase: 'staging' | 'committing' | 'indexing';
  done: number;
  total: number;
}

/** 0..1 for the progress bar: staging, then moving into the vault, then the index. */
export function progressFraction(p: ImportProgress | null): number {
  if (!p) return 0;
  const within = p.total > 0 ? Math.min(p.done / p.total, 1) : 0;
  if (p.phase === 'staging') return within * 0.45;
  if (p.phase === 'committing') return 0.45 + within * 0.45;
  return 0.95;
}

export function progressLabel(p: ImportProgress | null): string {
  if (!p) return 'Reading notes…';
  if (p.phase === 'staging') return `Preparing ${p.done} of ${p.total}`;
  if (p.phase === 'committing') return `Adding to the vault ${p.done} of ${p.total}`;
  return 'Updating the search index…';
}
