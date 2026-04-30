const TASK_LIST_ITEM_RE = /^(\s*)([-*+]|\d+\.)(\s+)\[([ xX])\](\s+)(.*)$/;

function isFence(line: string): string | null {
  const match = line.match(/^\s*(```+|~~~+)\s*$/);
  return match ? match[1] : null;
}

/**
 * Toggle the nth markdown task list item to the requested checked state.
 * Returns null when the item cannot be found.
 */
export function toggleTaskListItemAtIndex(
  content: string,
  targetIndex: number,
  checked: boolean,
): string | null {
  if (targetIndex < 0) return null;

  const lines = content.split('\n');
  let fence: string | null = null;
  let taskIndex = 0;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];

    const nextFence = isFence(line);
    if (nextFence) {
      fence = fence === nextFence ? null : nextFence;
      continue;
    }

    if (fence) continue;

    const match = line.match(TASK_LIST_ITEM_RE);
    if (!match) continue;

    if (taskIndex === targetIndex) {
      lines[i] = `${match[1]}${match[2]}${match[3]}[${checked ? 'x' : ' '}]${match[5]}${match[6]}`;
      return lines.join('\n');
    }

    taskIndex += 1;
  }

  return null;
}
