import { expect, test } from 'vitest';
import { toggleTaskListItemAtIndex } from './task-list';

test('toggleTaskListItemAtIndex toggles the requested task item', () => {
  const input = '- [ ] first\n- [x] second';

  expect(toggleTaskListItemAtIndex(input, 1, false)).toBe('- [ ] first\n- [ ] second');
});

test('toggleTaskListItemAtIndex returns null when the item is missing', () => {
  expect(toggleTaskListItemAtIndex('- [ ] only', 2, true)).toBeNull();
});
