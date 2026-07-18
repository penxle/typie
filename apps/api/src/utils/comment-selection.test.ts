import assert from 'node:assert/strict';
import test from 'node:test';
import { extractSelectionDots, normalizeStablePosition, normalizeStableSelection } from './comment-selection.ts';

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

test('extractSelectionDots reads current-format chain[]/child.dot positions', () => {
  const selection = {
    anchor: { chain: ['0_0', '3x_1A'], child: { dot: '3x_1B', bind: 'left' }, affinity: 'downstream' },
    head: { chain: ['0_0', '4y_2C'], child: null, affinity: 'upstream' },
  };

  assert.deepEqual(extractSelectionDots(selection), {
    kind: 'ok',
    dots: ['0_0', '3x_1A', '3x_1B', '4y_2C'],
  });
});

test('extractSelectionDots reads current-format positions with an implicit null child (no child key)', () => {
  const selection = {
    anchor: { chain: ['0_AzL8n0Y58m8', '1_1jh'], affinity: 'upstream' },
    head: { chain: ['0_AzL8n0Y58m8', '1_1Tw'], affinity: 'downstream' },
  };

  assert.deepEqual(extractSelectionDots(selection), {
    kind: 'ok',
    dots: ['0_AzL8n0Y58m8', '1_1jh', '1_1Tw'],
  });
});

test('extractSelectionDots reads legacy binding.anchor positions', () => {
  const selection = {
    anchor: { chain: ['0_0', '3x_1A'], binding: { type: 'adjacent', anchor: '3x_1B', bind: 'left' }, affinity: 'downstream' },
    head: { chain: ['0_0', '4y_2C'], binding: { type: 'container_start' }, affinity: 'downstream' },
  };

  assert.deepEqual(extractSelectionDots(selection), {
    kind: 'ok',
    dots: ['0_0', '3x_1A', '3x_1B', '4y_2C'],
  });
});

test('extractSelectionDots reads the pre-rewrite kind-tagged chain[].child_dot + char_dot positions', () => {
  const selection = {
    anchor: {
      kind: 'char',
      chain: [{ node_id: '0', child_dot: '0_0' }],
      offset: 23,
      bind: 'right',
      affinity: 'downstream',
      char_dot: 'L03pmlNIfyG_Eh',
    },
    head: { kind: 'container_start', chain: [{ node_id: '0', child_dot: '0_0' }], affinity: 'downstream' },
  };

  assert.deepEqual(extractSelectionDots(selection), {
    kind: 'ok',
    dots: ['0', '0_0', 'L03pmlNIfyG_Eh'],
  });
});

test('extractSelectionDots reads the untagged binding.Adjacent.anchor positions', () => {
  const selection = {
    anchor: { chain: ['0_0'], binding: { Adjacent: { anchor: '1_1', bind: 'Left' } }, affinity: 'downstream' },
    head: { chain: ['0_0'], binding: { Adjacent: { anchor: '2_2', bind: 'Right' } }, affinity: 'downstream' },
  };

  assert.deepEqual(extractSelectionDots(selection), {
    kind: 'ok',
    dots: ['0_0', '1_1', '2_2'],
  });
});

test('extractSelectionDots reports unrecognized when either side has an unrecognized shape', () => {
  const unrecognizedAnchor = {
    anchor: { chain: [{ node_id: '0' }], affinity: 'downstream' },
    head: { chain: ['0_0'], child: null, affinity: 'upstream' },
  };
  const unrecognizedHead = {
    anchor: { chain: ['0_0'], child: null, affinity: 'upstream' },
    head: 'garbage',
  };

  assert.deepEqual(extractSelectionDots(unrecognizedAnchor), { kind: 'unrecognized' });
  assert.deepEqual(extractSelectionDots(unrecognizedHead), { kind: 'unrecognized' });
  assert.deepEqual(extractSelectionDots('garbage'), { kind: 'unrecognized' });
  assert.deepEqual(extractSelectionDots(null), { kind: 'unrecognized' });
});
