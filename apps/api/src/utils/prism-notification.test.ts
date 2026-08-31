import assert from 'node:assert/strict';
import test from 'node:test';
import { prismRunNotification, prismUserActionNotification } from './prism-notification.ts';

test('completed and failed runs resolve with stable elapsed notification', () => {
  for (const state of ['COMPLETED', 'FAILED'] as const) {
    assert.deepEqual(prismRunNotification({ sessionId: 'session-1', runSeq: 4, state, startedAt: 1000, finishedAt: 32_500 }), {
      id: 'prism:notification:session-1:run:4:resolved',
      sessionId: 'session-1',
      kind: 'TURN_RESOLVED',
      elapsedMs: 31_500,
    });
  }
});

test('a user action resets the elapsed time for the resumed response', () => {
  const notification = prismRunNotification({
    sessionId: 'session-1',
    runSeq: 4,
    state: 'COMPLETED',
    startedAt: 1000,
    lastUserActionAt: 31_000,
    finishedAt: 32_500,
  });

  assert.equal(notification?.elapsedMs, 1500);
});

test('a user action outside the run does not change its elapsed time', () => {
  for (const lastUserActionAt of [500, 33_000]) {
    const notification = prismRunNotification({
      sessionId: 'session-1',
      runSeq: 4,
      state: 'COMPLETED',
      startedAt: 1000,
      lastUserActionAt,
      finishedAt: 32_500,
    });

    assert.equal(notification?.elapsedMs, 31_500);
  }
});

test('canceled and running runs do not produce a sound notification', () => {
  for (const state of ['CANCELED', 'RUNNING'] as const) {
    assert.equal(prismRunNotification({ sessionId: 'session-1', runSeq: 4, state, startedAt: 1000, finishedAt: 32_500 }), null);
  }
});

test('notification elapsed time never becomes negative', () => {
  assert.equal(
    prismRunNotification({ sessionId: 'session-1', runSeq: 4, state: 'COMPLETED', startedAt: 2000, finishedAt: 1000 })?.elapsedMs,
    0,
  );
});

test('user action notification is stable per tool call', () => {
  assert.deepEqual(prismUserActionNotification({ sessionId: 'session-1', toolCallId: 'tool-2', startedAt: 2000, requestedAt: 12_500 }), {
    id: 'prism:notification:session-1:tool:tool-2:action-required',
    sessionId: 'session-1',
    kind: 'USER_ACTION_REQUIRED',
    elapsedMs: 10_500,
  });
});

test('a user action resets the elapsed time for the next action request', () => {
  assert.equal(
    prismUserActionNotification({
      sessionId: 'session-1',
      toolCallId: 'tool-2',
      startedAt: 2000,
      lastUserActionAt: 12_000,
      requestedAt: 12_500,
    }).elapsedMs,
    500,
  );
});
