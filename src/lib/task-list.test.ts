// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { toggleTaskListItemAtIndex } from './task-list';

test('toggleTaskListItemAtIndex toggles the requested task item', () => {
  const input = '- [ ] first\n- [x] second';

  assert.equal(toggleTaskListItemAtIndex(input, 1, false), '- [ ] first\n- [ ] second');
});

test('toggleTaskListItemAtIndex returns null when the item is missing', () => {
  assert.equal(toggleTaskListItemAtIndex('- [ ] only', 2, true), null);
});
