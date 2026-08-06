import assert from 'node:assert/strict';
import test from 'node:test';
import { buildConcatScript, exceedsAnimatedImageBudget } from './animated-webp.ts';

test('buildConcatScript emits per-frame durations on a centisecond grid', () => {
  const script = buildConcatScript(['/t/frame_00000.png', '/t/frame_00001.png', '/t/frame_00002.png'], [40, 113, 200]);

  assert.equal(
    script,
    [
      'ffconcat version 1.0',
      "file '/t/frame_00000.png'",
      'option framerate 100',
      'duration 0.04',
      "file '/t/frame_00001.png'",
      'option framerate 100',
      'duration 0.07',
      "file '/t/frame_00002.png'",
      'option framerate 100',
      'duration 0.09',
      '',
    ].join('\n'),
  );
});

test('buildConcatScript clamps zero-delay frames to one centisecond', () => {
  const script = buildConcatScript(['/t/frame_00000.png', '/t/frame_00001.png'], [40, 40]);

  assert.match(script, /duration 0\.04\n.*\n.*\nduration 0\.01\n$/);
});

test('exceedsAnimatedImageBudget compares decoded RGBA size against the budget', () => {
  assert.equal(exceedsAnimatedImageBudget({ width: 1920, height: 1080, pages: 188 }), false);
  assert.equal(exceedsAnimatedImageBudget({ width: 3840, height: 2160, pages: 200 }), true);
  assert.equal(exceedsAnimatedImageBudget({ width: 1920, height: 1080 }), true);
});
