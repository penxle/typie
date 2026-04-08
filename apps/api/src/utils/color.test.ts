import assert from 'node:assert/strict';
import test from 'node:test';
import { applySvgRootColor, normalizeHexColor } from './color.ts';

test('normalizeHexColor canonicalizes valid colors', () => {
  assert.equal(normalizeHexColor('#f1f1f7'), '#F1F1F7');
});

test('normalizeHexColor returns null for invalid colors', () => {
  assert.equal(normalizeHexColor('not-a-color'), null);
});

test('applySvgRootColor injects or replaces the svg root color attribute', () => {
  assert.equal(
    applySvgRootColor('<svg xmlns="http://www.w3.org/2000/svg"></svg>', '#F1F1F7'),
    '<svg color="#F1F1F7" xmlns="http://www.w3.org/2000/svg"></svg>',
  );

  assert.equal(
    applySvgRootColor('<svg color="#000000" xmlns="http://www.w3.org/2000/svg"></svg>', '#F1F1F7'),
    '<svg color="#F1F1F7" xmlns="http://www.w3.org/2000/svg"></svg>',
  );
});
