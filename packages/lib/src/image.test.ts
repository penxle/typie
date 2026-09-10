import assert from 'node:assert/strict';
import test from 'node:test';
import { fitImageSize } from './image.ts';

test('fits tall images into the page while preserving aspect ratio', () => {
  assert.deepEqual(fitImageSize({ width: 600, height: 3000, maxWidth: 600, maxHeight: 800 }), { width: 160, height: 800 });
});

test('applies the proportion to the fitted size without enlarging the original', () => {
  assert.deepEqual(fitImageSize({ width: 600, height: 3000, maxWidth: 600, maxHeight: 800, proportion: 0.5 }), {
    width: 80,
    height: 400,
  });
  assert.deepEqual(fitImageSize({ width: 320, height: 240, maxWidth: 600, maxHeight: 800 }), { width: 320, height: 240 });
  assert.deepEqual(fitImageSize({ width: 320, height: 240, maxWidth: 600, proportion: 0.5 }), { width: 160, height: 120 });
});

test('allows a very narrow image to shrink to ten percent of its fitted size', () => {
  const dimensions = { width: 600, height: 20_000, maxWidth: 600, maxHeight: 800 };
  const minimum = fitImageSize({ ...dimensions, proportion: 0.1 });
  assert.equal(minimum.height, 80);
  assert.ok(Math.abs(minimum.width - 2.4) < 0.0001);
  assert.deepEqual(fitImageSize({ ...dimensions, proportion: 0 }), minimum);
  assert.deepEqual(fitImageSize({ ...dimensions, proportion: 2 }), fitImageSize(dimensions));
});

test('does not limit the height of a continuous image', () => {
  assert.deepEqual(fitImageSize({ width: 600, height: 3000, maxWidth: 600 }), { width: 600, height: 3000 });
});
