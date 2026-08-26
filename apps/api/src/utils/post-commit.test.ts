import assert from 'node:assert/strict';
import test from 'node:test';
import { runAfterCommit, runPostCommitEffects } from './post-commit.ts';

test('runAfterCommit registers effects until the owner runs them', async () => {
  const effects: (() => void | Promise<void>)[] = [];
  const order: string[] = [];
  const afterCommit = (effect: () => void | Promise<void>) => {
    effects.push(effect);
  };

  order.push('write');
  await runAfterCommit(afterCommit, async () => {
    order.push('publish-1');
  });
  await runAfterCommit(afterCommit, () => {
    order.push('publish-2');
  });
  order.push('ledger');
  assert.deepEqual(order, ['write', 'ledger']);

  const errors = await runPostCommitEffects(effects);
  assert.deepEqual(errors, []);
  assert.deepEqual(order, ['write', 'ledger', 'publish-1', 'publish-2']);
});

test('runAfterCommit runs immediately when there is no outer transaction', async () => {
  let ran = false;
  await runAfterCommit(undefined, () => {
    ran = true;
  });

  assert.equal(ran, true);
});

test('post-commit effect failures do not skip the remaining effects', async () => {
  const order: string[] = [];
  const errors = await runPostCommitEffects([
    () => {
      order.push('first');
      throw new Error('enqueue failed');
    },
    () => {
      order.push('second');
    },
  ]);

  assert.deepEqual(order, ['first', 'second']);
  assert.equal(errors.length, 1);
  assert.match(String(errors[0]), /enqueue failed/);
});
