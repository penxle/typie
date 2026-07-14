import assert from 'node:assert/strict';
import test from 'node:test';
import { normalizeStablePosition, normalizeStableSelection } from './comment-selection.ts';

test('normalizeStablePosition converts legacy adjacent binding to child', () => {
  const position = {
    chain: ['0_0', '3x_1A'],
    binding: { type: 'adjacent', anchor: '3x_1B', bind: 'left' },
    affinity: 'downstream',
  };

  assert.deepEqual(normalizeStablePosition(position), {
    kind: 'normalized',
    value: { chain: ['0_0', '3x_1A'], child: { dot: '3x_1B', bind: 'left' }, affinity: 'downstream' },
  });
});

test('normalizeStablePosition converts legacy container_start binding to null child', () => {
  const position = {
    chain: ['0_0', '4y_2C'],
    binding: { type: 'container_start' },
    affinity: 'downstream',
  };

  assert.deepEqual(normalizeStablePosition(position), {
    kind: 'normalized',
    value: { chain: ['0_0', '4y_2C'], child: null, affinity: 'downstream' },
  });
});

test('normalizeStablePosition restores missing child key as explicit null', () => {
  const position = { chain: ['0_AzL8n0Y58m8', '1_1jh'], affinity: 'upstream' };

  assert.deepEqual(normalizeStablePosition(position), {
    kind: 'normalized',
    value: { chain: ['0_AzL8n0Y58m8', '1_1jh'], child: null, affinity: 'upstream' },
  });
});

test('normalizeStablePosition keeps current-format positions', () => {
  assert.deepEqual(normalizeStablePosition({ chain: ['0_0'], child: { dot: '1_1', bind: 'right' }, affinity: 'downstream' }), {
    kind: 'keep',
  });
  assert.deepEqual(normalizeStablePosition({ chain: ['0_0'], child: null, affinity: 'upstream' }), { kind: 'keep' });
});

test('normalizeStablePosition rejects unrecognized shapes', () => {
  const kindTagged = {
    kind: 'char',
    chain: [{ node_id: '0', child_dot: '0_0' }],
    offset: 23,
    bind: 'right',
    affinity: 'downstream',
    char_dot: 'L03pmlNIfyG_Eh',
  };
  const untaggedBinding = { chain: ['0_0'], binding: { Adjacent: { anchor: '1_1', bind: 'Left' } }, affinity: 'downstream' };

  assert.deepEqual(normalizeStablePosition(kindTagged), { kind: 'unrecognized' });
  assert.deepEqual(normalizeStablePosition(untaggedBinding), { kind: 'unrecognized' });
  assert.deepEqual(normalizeStablePosition(null), { kind: 'unrecognized' });
  assert.deepEqual(normalizeStablePosition({ chain: [{ node_id: '0' }], affinity: 'downstream' }), { kind: 'unrecognized' });
});

test('normalizeStableSelection normalizes only the sides that need it', () => {
  const anchor = { chain: ['0_AzL8n0Y58m8', '1_1Tw'], child: { dot: '1_1Tx', bind: 'left' }, affinity: 'downstream' };
  const selection = {
    anchor,
    head: { chain: ['0_AzL8n0Y58m8', '1_1jh'], affinity: 'upstream' },
  };

  assert.deepEqual(normalizeStableSelection(selection), {
    anchor,
    head: { chain: ['0_AzL8n0Y58m8', '1_1jh'], child: null, affinity: 'upstream' },
  });
});

test('normalizeStableSelection returns the input as-is when nothing needs normalization', () => {
  const selection = {
    anchor: { chain: ['0_0'], child: null, affinity: 'downstream' },
    head: { chain: ['0_0'], child: { dot: '1_1', bind: 'right' }, affinity: 'upstream' },
  };

  assert.equal(normalizeStableSelection(selection), selection);
});

test('normalizeStableSelection passes through unrecognized selections untouched', () => {
  const selection = {
    anchor: { kind: 'container_start', chain: [{ node_id: '0', child_dot: '0_0' }], affinity: 'downstream' },
    head: { kind: 'char', chain: [{ node_id: '0', child_dot: '0_0' }], offset: 1, bind: 'right', affinity: 'downstream' },
  };

  assert.equal(normalizeStableSelection(selection), selection);
  assert.equal(normalizeStableSelection('garbage'), 'garbage');
});
