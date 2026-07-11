import assert from 'node:assert/strict';
import test from 'node:test';
import { compareStreamSeq, decodeClientMessage, decodeRaw, encodeMessage } from './protocol.ts';
import type { ClientMessage } from './protocol.ts';

test('클라이언트 메시지 CBOR 왕복', () => {
  const bytes = Uint8Array.of(1, 2, 3);
  const encoded = encodeMessage({ t: 'push', id: 'r1', documentId: 'D1', changesets: bytes });
  const result = decodeClientMessage(encoded);
  assert.ok(result.ok);
  assert.equal(result.message.t, 'push');
  if (result.message.t !== 'push') return;
  assert.equal(result.message.id, 'r1');
  assert.deepEqual(new Uint8Array(result.message.changesets), bytes);
});

test('모든 클라이언트 메시지 타입이 왕복 가능', () => {
  const messages: ClientMessage[] = [
    { t: 'hello', ticket: 'tk', clientId: 'c1', capabilities: [] },
    { t: 'ping' },
    { t: 'attach', documentId: 'D1' },
    { t: 'attach', documentId: 'D1', sinceSeq: '5-0' },
    { t: 'attach', documentId: 'D1', snapshotCursor: { rowId: 'B1', seq: 3, offset: 1024 } },
    { t: 'detach', documentId: 'D1' },
    { t: 'pull', id: 'r2', documentId: 'D1', sinceSeq: '5-0' },
  ];
  for (const message of messages) {
    const result = decodeClientMessage(encodeMessage(message));
    assert.ok(result.ok, message.t);
    assert.equal(result.message.t, message.t);
  }
});

test('서버 메시지는 표준 CBOR 맵으로 인코딩된다 (record 확장 없음)', () => {
  const encoded = encodeMessage({ t: 'pong' });
  const raw = decodeRaw(encoded) as Record<string, unknown>;
  assert.equal(raw.t, 'pong');
  assert.equal(encoded[0] >> 5, 5);
});

test('모르는 t는 unknown(계측용 type 포함), 깨진 바이트는 malformed', () => {
  assert.deepEqual(decodeClientMessage(encodeMessage({ t: 'brand-new' } as never)), { ok: false, reason: 'unknown', type: 'brand-new' });
  assert.deepEqual(decodeClientMessage(Uint8Array.of(0xff, 0x00)), { ok: false, reason: 'malformed' });
});

test('compareStreamSeq는 Redis stream id를 숫자 부분별로 비교', () => {
  assert.equal(compareStreamSeq('2-0', '10-0') < 0, true);
  assert.equal(compareStreamSeq('10-2', '10-10') < 0, true);
  assert.equal(compareStreamSeq('10-1', '10-1'), 0);
});

test('cursor 필드는 정수만 허용', () => {
  const fractional = encodeMessage({ t: 'attach', documentId: 'D1', snapshotCursor: { rowId: 'B1', seq: 1.5, offset: 0 } } as never);
  assert.deepEqual(decodeClientMessage(fractional), { ok: false, reason: 'malformed' });
  const negative = encodeMessage({ t: 'attach', documentId: 'D1', snapshotCursor: { rowId: 'B1', seq: 1, offset: -1 } } as never);
  assert.deepEqual(decodeClientMessage(negative), { ok: false, reason: 'malformed' });
});

test('sinceSeq는 빈 문자열 또는 stream id 형식만 유효', () => {
  const bad = encodeMessage({ t: 'pull', id: 'r1', documentId: 'D1', sinceSeq: 'not-a-seq' } as never);
  assert.deepEqual(decodeClientMessage(bad), { ok: false, reason: 'malformed' });
  const empty = decodeClientMessage(encodeMessage({ t: 'attach', documentId: 'D1', sinceSeq: '' }));
  assert.ok(empty.ok);
  const valid = decodeClientMessage(encodeMessage({ t: 'pull', id: 'r1', documentId: 'D1', sinceSeq: '123-4' }));
  assert.ok(valid.ok);
});

test('필수 필드 누락은 malformed', () => {
  const encoded = encodeMessage({ t: 'push', id: 'r1' } as never);
  assert.deepEqual(decodeClientMessage(encoded), { ok: false, reason: 'malformed' });
});

test('모르는 추가 키는 무시된다 (전방 호환)', () => {
  const result = decodeClientMessage(encodeMessage({ t: 'ping', extra: 'future-field' } as never));
  assert.ok(result.ok);
  assert.equal(result.message.t, 'ping');
});

const CONTRACT_PUSH_HEX = 'b90004617464707573686269646272316a646f63756d656e7449646244316a6368616e676573657473d84043010203';

test('클라이언트 계약 벡터: 고정 바이트가 push로 디코드된다', () => {
  const bytes = Uint8Array.from(Buffer.from(CONTRACT_PUSH_HEX, 'hex'));
  const result = decodeClientMessage(bytes);
  assert.ok(result.ok);
  assert.equal(result.message.t, 'push');
  if (result.message.t !== 'push') return;
  assert.equal(result.message.id, 'r1');
  assert.deepEqual(new Uint8Array(result.message.changesets), Uint8Array.of(1, 2, 3));
});

const CONTRACT_SNAPSHOT_END_HEX =
  'b9000561746c736e617073686f742d656e646a646f63756d656e7449646244316373657163352d30656865616473d84041096c64757261626c654865616473d84040';

test('서버 계약 벡터: snapshot-end 인코딩이 고정 바이트와 일치한다', () => {
  const encoded = encodeMessage({
    t: 'snapshot-end',
    documentId: 'D1',
    seq: '5-0',
    heads: Uint8Array.of(9),
    durableHeads: new Uint8Array(),
  });
  assert.equal(Buffer.from(encoded).toString('hex'), CONTRACT_SNAPSHOT_END_HEX);
});

const KOTLIN_PUSH_HEX = 'bf617464707573686269646272316a646f63756d656e7449646244316a6368616e67657365747343010203ff';

test('Kotlin 계약 벡터: kotlinx-serialization-cbor이 인코드한 push가 디코드된다', () => {
  const bytes = Uint8Array.from(Buffer.from(KOTLIN_PUSH_HEX, 'hex'));
  const result = decodeClientMessage(bytes);
  assert.ok(result.ok);
  assert.equal(result.message.t, 'push');
  if (result.message.t !== 'push') return;
  assert.equal(result.message.id, 'r1');
  assert.equal(result.message.documentId, 'D1');
  assert.deepEqual(new Uint8Array(result.message.changesets), Uint8Array.of(1, 2, 3));
});
