import { Encoder } from 'cbor-x';
import { describe, expect, test } from 'vitest';
import { compareStreamSeq, decodeServerMessage, encodeClientMessage } from './protocol';

const serverEncoder = new Encoder({ useRecords: false });
const serverBytes = (message: unknown): Uint8Array => serverEncoder.encode(message);

describe('protocol codec', () => {
  test('클라이언트 메시지가 표준 CBOR 맵으로 인코딩된다', () => {
    const encoded = encodeClientMessage({ t: 'ping' });
    expect(encoded[0] >> 5).toBe(5);
  });

  test('서버 메시지 디코드: 알려진 타입', () => {
    const encoded = encodeClientMessage({ t: 'ping' } as never);
    const decoded = decodeServerMessage(encoded);
    expect(decoded).toBeNull();
    const pong = decodeServerMessage(serverBytes({ t: 'pong' }));
    expect(pong?.t).toBe('pong');
  });

  test('모르는 t·깨진 프레임은 null (asset 타입 허용목록 추가 후에도)', () => {
    expect(decodeServerMessage(serverBytes({ t: 'brand-new' }))).toBeNull();
    expect(decodeServerMessage(serverBytes({ t: 'asset-brand-new' }))).toBeNull();
    expect(decodeServerMessage(Uint8Array.of(0xff, 0x00))).toBeNull();
  });

  test('asset-state 3상태의 encode→decode 왕복', () => {
    const missing = decodeServerMessage(
      serverBytes({ t: 'asset-state', documentId: 'D1', requestId: 'q1', assets: [{ id: 'IMG01', state: 'missing' }], final: false }),
    );
    if (missing?.t !== 'asset-state') throw new Error('unexpected');
    expect(missing.requestId).toBe('q1');
    expect(missing.final).toBe(false);
    expect(missing.assets).toEqual([{ id: 'IMG01', state: 'missing' }]);

    const pending = decodeServerMessage(
      serverBytes({
        t: 'asset-state',
        documentId: 'D1',
        requestId: 'q2',
        assets: [{ id: 'IMG02', state: 'pending', meta: { kind: 'image', name: 'a.png', size: 10 } }],
        final: false,
      }),
    );
    if (pending?.t !== 'asset-state') throw new Error('unexpected');
    expect(pending.assets).toEqual([{ id: 'IMG02', state: 'pending', meta: { kind: 'image', name: 'a.png', size: 10 } }]);

    const ready = decodeServerMessage(
      serverBytes({
        t: 'asset-state',
        documentId: 'D1',
        requestId: 'q3',
        assets: [
          {
            id: 'IMG03',
            state: 'ready',
            asset: { type: 'image', id: 'IMG03', url: 'u', originalUrl: 'o', width: 1, height: 1, placeholder: null },
          },
        ],
        final: true,
      }),
    );
    if (ready?.t !== 'asset-state') throw new Error('unexpected');
    expect(ready.final).toBe(true);
    expect(ready.assets).toEqual([
      {
        id: 'IMG03',
        state: 'ready',
        asset: { type: 'image', id: 'IMG03', url: 'u', originalUrl: 'o', width: 1, height: 1, placeholder: null },
      },
    ]);
  });

  test('asset-changed의 encode→decode 왕복', () => {
    const decoded = decodeServerMessage(serverBytes({ t: 'asset-changed', documentId: 'D1', ids: ['IMG01', 'IMG02'] }));
    if (decoded?.t !== 'asset-changed') throw new Error('unexpected');
    expect(decoded.documentId).toBe('D1');
    expect(decoded.ids).toEqual(['IMG01', 'IMG02']);
  });

  test('바이너리 필드 왕복', () => {
    const bytes = Uint8Array.of(1, 2, 3);
    const decoded = decodeServerMessage(serverBytes({ t: 'push-ack', id: 'r1', heads: bytes, durableHeads: new Uint8Array() }));
    if (decoded?.t !== 'push-ack') throw new Error('unexpected');
    expect(new Uint8Array(decoded.heads)).toEqual(bytes);
  });

  test('compareStreamSeq는 숫자 부분별 비교', () => {
    expect(compareStreamSeq('2-0', '10-0')).toBeLessThan(0);
    expect(compareStreamSeq('10-2', '10-10')).toBeLessThan(0);
    expect(compareStreamSeq('10-1', '10-1')).toBe(0);
  });

  const CONTRACT_PUSH_HEX = 'b90004617464707573686269646272316a646f63756d656e7449646244316a6368616e676573657473d84043010203';

  test('서버 계약 벡터: push 인코딩이 고정 바이트와 일치', () => {
    const encoded = encodeClientMessage({ t: 'push', id: 'r1', documentId: 'D1', changesets: Uint8Array.of(1, 2, 3) });
    expect(Buffer.from(encoded).toString('hex')).toBe(CONTRACT_PUSH_HEX);
  });

  const CONTRACT_SNAPSHOT_END_HEX =
    'b9000561746c736e617073686f742d656e646a646f63756d656e7449646244316373657163352d30656865616473d84041096c64757261626c654865616473d84040';

  test('서버 계약 벡터: snapshot-end 고정 바이트를 디코드한다', () => {
    const bytes = Uint8Array.from(Buffer.from(CONTRACT_SNAPSHOT_END_HEX, 'hex'));
    const decoded = decodeServerMessage(bytes);
    if (decoded?.t !== 'snapshot-end') throw new Error('unexpected');
    expect(decoded.seq).toBe('5-0');
    expect(new Uint8Array(decoded.heads)).toEqual(Uint8Array.of(9));
  });

  const CONTRACT_ASSET_PULL_HEX =
    'b9000461746a61737365742d70756c6c6a646f63756d656e74496462443169726571756573744964627131636964738165494d473031';

  test('서버 계약 벡터: asset-pull 인코딩이 고정 바이트와 일치', () => {
    const encoded = encodeClientMessage({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });
    expect(Buffer.from(encoded).toString('hex')).toBe(CONTRACT_ASSET_PULL_HEX);
  });

  const CONTRACT_ASSET_STATE_HEX =
    'b9000561746b61737365742d73746174656a646f63756d656e744964624431697265717565737449646271316661737365747381b9000262696465494d473031657374617465676d697373696e676566696e616cf5';

  test('서버 계약 벡터: asset-state 고정 바이트를 디코드한다', () => {
    const bytes = Uint8Array.from(Buffer.from(CONTRACT_ASSET_STATE_HEX, 'hex'));
    const decoded = decodeServerMessage(bytes);
    if (decoded?.t !== 'asset-state') throw new Error('unexpected');
    expect(decoded.documentId).toBe('D1');
    expect(decoded.requestId).toBe('q1');
    expect(decoded.assets).toEqual([{ id: 'IMG01', state: 'missing' }]);
    expect(decoded.final).toBe(true);
  });

  const CONTRACT_ASSET_CHANGED_HEX = 'b9000361746d61737365742d6368616e6765646a646f63756d656e744964624431636964738165494d473031';

  test('서버 계약 벡터: asset-changed 고정 바이트를 디코드한다', () => {
    const bytes = Uint8Array.from(Buffer.from(CONTRACT_ASSET_CHANGED_HEX, 'hex'));
    const decoded = decodeServerMessage(bytes);
    if (decoded?.t !== 'asset-changed') throw new Error('unexpected');
    expect(decoded.documentId).toBe('D1');
    expect(decoded.ids).toEqual(['IMG01']);
  });
});
