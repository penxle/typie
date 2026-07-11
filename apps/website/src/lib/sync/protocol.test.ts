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

  test('모르는 t·깨진 프레임은 null', () => {
    expect(decodeServerMessage(serverBytes({ t: 'brand-new' }))).toBeNull();
    expect(decodeServerMessage(Uint8Array.of(0xff, 0x00))).toBeNull();
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
});
