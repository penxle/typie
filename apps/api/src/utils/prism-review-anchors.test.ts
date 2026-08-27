import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveOutcomeAnchors } from './prism-review-anchors.ts';
import { unresolvedOutcomeAnchors } from './prism-review-core.ts';
import type { ProseAnchorCapture, ProseRange, StableSelection } from '@typie/editor-ffi/server';
import type { ReviewOutcome } from '@typie/prism';
import type { AnchorCaptureDeps, AnchorCaptureFailure } from './prism-review-anchors.ts';

const content = '가나다 라마바 사아자';
const heads = new Uint8Array([1, 2, 3]);
const graph = new Uint8Array([9, 9]);

const outcomeWith = (anchors: { head: string; tail: string }[]): ReviewOutcome => ({
  version: 1,
  kind: 'issues',
  issues: [{ trait: 't', pass: 'judgment', body: null, anchors }],
});

const outcome = outcomeWith([
  { head: '가나다', tail: '라마바' },
  { head: '없는말', tail: '없는말' },
]);

// wasm이 structured clone으로 넘기는 형태 — child 키가 빠진 채 온다
const selection = {
  version: 2,
  anchor: { chain: [], child: undefined, affinity: 'downstream' },
  head: { chain: [], child: undefined, affinity: 'upstream' },
} as StableSelection;

type CaptureCall = { graph: Uint8Array; heads: Uint8Array; expectedText: string; ranges: ProseRange[] };

const harness = (behavior: { readGraph?: () => Promise<Uint8Array>; capture?: () => Promise<ProseAnchorCapture> } = {}) => {
  const calls = { readGraph: 0, capture: [] as CaptureCall[], report: [] as AnchorCaptureFailure[] };
  const deps: AnchorCaptureDeps = {
    readGraph: () => {
      calls.readGraph += 1;
      return behavior.readGraph ? behavior.readGraph() : Promise.resolve(graph);
    },
    capture: (g, h, expectedText, ranges) => {
      calls.capture.push({ graph: g, heads: h, expectedText, ranges });
      return behavior.capture ? behavior.capture() : Promise.resolve({ text_matches: true, anchors: [] });
    },
    report: (failure) => {
      calls.report.push(failure);
    },
  };

  return { deps, calls };
};

test('resolveOutcomeAnchors: 매칭된 앵커만 캡처에 보내고 히트를 자리로 되돌리며 selection의 child를 복원한다', async () => {
  const { deps, calls } = harness({
    capture: () => Promise.resolve({ text_matches: true, anchors: [{ index: 0, selection, text: '가나다 라마바' }] }),
  });

  const anchors = await resolveOutcomeAnchors(outcome, { content, heads }, deps);

  assert.equal(calls.readGraph, 1);
  assert.equal(calls.capture.length, 1);
  assert.equal(calls.capture[0].graph, graph);
  assert.equal(calls.capture[0].heads, heads);
  assert.equal(calls.capture[0].expectedText, content);
  assert.equal(calls.capture[0].ranges.length, 1);
  assert.deepEqual(calls.capture[0].ranges[0], { start: 0, end: 7 });
  assert.deepStrictEqual(anchors.issues[0][0], {
    head: '가나다',
    tail: '라마바',
    selection: {
      version: 2,
      anchor: { chain: [], child: null, affinity: 'downstream' },
      head: { chain: [], child: null, affinity: 'upstream' },
    },
    text: '가나다 라마바',
  });
  assert.deepStrictEqual(anchors.issues[0][1], { head: '없는말', tail: '없는말', selection: null, text: null });
  assert.deepEqual(calls.report, []);
});

test('resolveOutcomeAnchors: 텍스트 게이트가 어긋나면 전건을 접고 실패를 알린다', async () => {
  const { deps, calls } = harness({ capture: () => Promise.resolve({ text_matches: false, anchors: [] }) });

  const anchors = await resolveOutcomeAnchors(outcome, { content, heads }, deps);

  assert.deepEqual(anchors, unresolvedOutcomeAnchors(outcome));
  assert.deepEqual(calls.report, [{ kind: 'text_mismatch' }]);
});

test('resolveOutcomeAnchors: 캡처가 던지면 전건을 접고 그 에러를 그대로 알린다', async () => {
  const error = new Error('capture boom');
  const { deps, calls } = harness({ capture: () => Promise.reject(error) });

  const anchors = await resolveOutcomeAnchors(outcome, { content, heads }, deps);

  assert.deepEqual(anchors, unresolvedOutcomeAnchors(outcome));
  assert.equal(calls.report.length, 1);
  assert.deepEqual(calls.report[0], { kind: 'capture_failed', error });
});

test('resolveOutcomeAnchors: 그래프를 못 읽으면 캡처까지 가지 않고 전건을 접는다', async () => {
  const error = new Error('graph boom');
  const { deps, calls } = harness({ readGraph: () => Promise.reject(error) });

  const anchors = await resolveOutcomeAnchors(outcome, { content, heads }, deps);

  assert.equal(calls.capture.length, 0);
  assert.deepEqual(anchors, unresolvedOutcomeAnchors(outcome));
  assert.deepEqual(calls.report[0], { kind: 'capture_failed', error });
});

test('resolveOutcomeAnchors: 앵커 자리가 없으면 외부 효과를 부르지 않는다', async () => {
  const empty = outcomeWith([]);
  const { deps, calls } = harness();

  const anchors = await resolveOutcomeAnchors(empty, { content, heads }, deps);

  assert.equal(calls.readGraph, 0);
  assert.equal(calls.capture.length, 0);
  assert.deepEqual(anchors, unresolvedOutcomeAnchors(empty));
  assert.deepEqual(calls.report, []);
});

test('resolveOutcomeAnchors: 전부 매칭에 실패하면 외부 효과 없이 전건을 접는다', async () => {
  const missing = outcomeWith([
    { head: '없는말', tail: '없는말' },
    { head: '이것도', tail: '없다' },
  ]);
  const { deps, calls } = harness();

  const anchors = await resolveOutcomeAnchors(missing, { content, heads }, deps);

  assert.equal(calls.readGraph, 0);
  assert.equal(calls.capture.length, 0);
  assert.deepEqual(anchors, unresolvedOutcomeAnchors(missing));
  assert.deepEqual(calls.report, []);
});

test('resolveOutcomeAnchors: 범위 밖 index의 히트는 버린다', async () => {
  const { deps, calls } = harness({
    capture: () => Promise.resolve({ text_matches: true, anchors: [{ index: 7, selection, text: 'x' }] }),
  });

  const anchors = await resolveOutcomeAnchors(outcome, { content, heads }, deps);

  assert.deepEqual(anchors, unresolvedOutcomeAnchors(outcome));
  assert.deepEqual(calls.report, []);
});
