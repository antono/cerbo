import { DiffFile } from '@git-diff-view/core';

export type ExternalPageChangeAction = 'reload' | 'prompt' | 'ignore';

export type PageDiffData = {
  oldFile: {
    fileName: string;
    fileLang: string;
    content: string;
  };
  newFile: {
    fileName: string;
    fileLang: string;
    content: string;
  };
  hunks: string[];
};

export type PageDiffFile = InstanceType<typeof DiffFile>;

export function pageMdPathToSlug(vaultPath: string, changedPath: string): string | null {
  const normalizedVault = vaultPath.replace(/\\/g, '/').replace(/\/$/, '');
  const normalizedChanged = changedPath.replace(/\\/g, '/');

  if (!normalizedChanged.startsWith(normalizedVault + '/')) return null;

  const relative = normalizedChanged.slice(normalizedVault.length + 1);
  const parts = relative.split('/');
  if (parts.length !== 2 || parts[1] !== 'page.md') return null;
  return parts[0] || null;
}

export function decideExternalPageChange(input: {
  currentSlug: string | null;
  changedSlug: string | null;
  editorTab: 'write' | 'preview';
  dirty: boolean;
}): ExternalPageChangeAction {
  if (!input.currentSlug || !input.changedSlug) return 'ignore';
  if (input.currentSlug !== input.changedSlug) return 'ignore';
  if (input.editorTab === 'preview') return 'reload';
  return 'prompt';
}

export function pageChangeKey(vaultId: string, path: string): string {
  return `${vaultId}:${path}`;
}

export function shouldSkipExternalPageChange(eventKey: string, suppressedKey: string | null): boolean {
  return suppressedKey !== null && suppressedKey === eventKey;
}

export function shouldIgnoreUnchangedPageChange(currentContent: string, diskContent: string): boolean {
  return currentContent === diskContent;
}

export function buildPageContentDiff(currentContent: string, diskContent: string, fileName = 'page.md'): string | null {
  if (currentContent === diskContent) return null;

  const currentLines = currentContent.split('\n');
  const diskLines = diskContent.split('\n');
  const maxPrefix = Math.min(currentLines.length, diskLines.length);
  let start = 0;

  while (start < maxPrefix && currentLines[start] === diskLines[start]) {
    start += 1;
  }

  let endCurrent = currentLines.length - 1;
  let endDisk = diskLines.length - 1;
  while (endCurrent >= start && endDisk >= start && currentLines[endCurrent] === diskLines[endDisk]) {
    endCurrent -= 1;
    endDisk -= 1;
  }

  const removed = currentLines.slice(start, endCurrent + 1);
  const added = diskLines.slice(start, endDisk + 1);

  return [
    `--- a/${fileName}`,
    `+++ b/${fileName}`,
    `@@ -${start + 1},${removed.length} +${start + 1},${added.length} @@`,
    ...removed.map((line) => `-${line}`),
    ...added.map((line) => `+${line}`),
  ].join('\n');
}

export function buildPageDiffData(currentContent: string, diskContent: string, fileName = 'page.md'): PageDiffData | null {
  const diff = buildPageContentDiff(currentContent, diskContent, fileName);
  if (!diff) return null;

  return {
    oldFile: {
      fileName,
      fileLang: 'markdown',
      content: currentContent,
    },
    newFile: {
      fileName,
      fileLang: 'markdown',
      content: diskContent,
    },
    hunks: [diff],
  };
}

export function buildPageDiffFile(currentContent: string, diskContent: string, fileName = 'page.md'): PageDiffFile | null {
  const diff = buildPageContentDiff(currentContent, diskContent, fileName);
  if (!diff) return null;

  const diffFile = DiffFile.createInstance({
    oldFile: {
      fileName,
      fileLang: 'markdown',
      content: currentContent,
    },
    newFile: {
      fileName,
      fileLang: 'markdown',
      content: diskContent,
    },
    hunks: [diff],
  });

  diffFile.initTheme('dark');
  diffFile.init();
  diffFile.buildUnifiedDiffLines();
  return diffFile as PageDiffFile;
}

export async function loadPageDiffData(
  readDiskContent: () => Promise<string>,
  currentContent: string,
  fileName = 'page.md',
): Promise<PageDiffData | null> {
  const diskContent = await readDiskContent();
  return buildPageDiffData(currentContent, diskContent, fileName);
}

export async function loadPageDiffFile(
  readDiskContent: () => Promise<string>,
  currentContent: string,
  fileName = 'page.md',
): Promise<PageDiffFile | null> {
  const diskContent = await readDiskContent();
  return buildPageDiffFile(currentContent, diskContent, fileName);
}

export function logPageContentDiff(debugLabel: string, currentContent: string, diskContent: string) {
  if (currentContent === diskContent) return;

  const diff = buildPageContentDiff(currentContent, diskContent);
  if (!diff) return;

  const lines = diff.split('\n');
  if (typeof console.groupCollapsed === 'function') {
    console.groupCollapsed(`%c${debugLabel}`, 'color:#7c3aed;font-weight:600');
  } else {
    console.log(debugLabel);
  }

  for (const line of lines) {
    if (line.startsWith('+') && !line.startsWith('+++')) {
      console.log(`%c${line}`, 'color:#16a34a');
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      console.log(`%c${line}`, 'color:#dc2626');
    } else if (line.startsWith('@@')) {
      console.log(`%c${line}`, 'color:#0f766e');
    } else {
      console.log(line);
    }
  }

  if (typeof console.groupEnd === 'function') {
    console.groupEnd();
  }
}
