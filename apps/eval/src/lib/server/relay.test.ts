import { describe, expect, it } from 'vitest';
import { resolveCursor, watchdogPipe } from './relay.ts';

const drain = async (stream: ReadableStream<Uint8Array>): Promise<string> => {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let out = '';
  for (;;) {
    const { done, value } = await reader.read();
    if (done) return out;
    out += decoder.decode(value, { stream: true });
  }
};

describe('watchdogPipe', () => {
  it('바이트를 그대로 통과시키고 업스트림 종료 시 닫힌다', async () => {
    const upstream = new ReadableStream<Uint8Array>({
      start(c) {
        c.enqueue(new TextEncoder().encode(': hb\n\n'));
        c.enqueue(new TextEncoder().encode('id: 1\nevent: run.started\ndata: {}\n\n'));
        c.close();
      },
    });
    await expect(drain(watchdogPipe(upstream, 45_000))).resolves.toContain('run.started');
  });

  it('프레임 바이트를 한 글자도 고치지 않는다', async () => {
    const raw = ': hb\n\nid: 7\nevent: step.finished\ndata: {"a":"b\\nc"}\n\n';
    const upstream = new ReadableStream<Uint8Array>({
      start(c) {
        c.enqueue(new TextEncoder().encode(raw));
        c.close();
      },
    });
    await expect(drain(watchdogPipe(upstream, 45_000))).resolves.toBe(raw);
  });

  it('무수신이 한도를 넘으면 스트림을 닫는다', async () => {
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- 빈 start가 곧 "영원히 무음"이다
    const upstream = new ReadableStream<Uint8Array>({ start() {} });
    await expect(drain(watchdogPipe(upstream, 20))).resolves.toBe('');
  });

  it('무수신으로 닫을 때 업스트림도 취소한다', async () => {
    let canceled = false;
    const upstream = new ReadableStream<Uint8Array>({
      cancel() {
        canceled = true;
      },
    });
    await expect(drain(watchdogPipe(upstream, 20))).resolves.toBe('');
    expect(canceled).toBe(true);
  });

  // 한도는 총 데드라인이 아니라 "마지막 수신 이후"다 — 청크마다 타이머가 리셋되지 않으면
  // 15ms 간격 3청크(누적 45ms)가 한도 40ms에서 잘린다.
  it('청크를 받을 때마다 무수신 한도를 리셋한다', async () => {
    const encoder = new TextEncoder();
    const upstream = new ReadableStream<Uint8Array>({
      async start(c) {
        for (const n of [1, 2, 3]) {
          await new Promise((resolve) => setTimeout(resolve, 15));
          c.enqueue(encoder.encode(`data: ${n}\n\n`));
        }
        c.close();
      },
    });
    await expect(drain(watchdogPipe(upstream, 40))).resolves.toBe('data: 1\n\ndata: 2\n\ndata: 3\n\n');
  });

  it('다운스트림 취소를 업스트림으로 전파한다', async () => {
    let canceled = false;
    const upstream = new ReadableStream<Uint8Array>({
      cancel() {
        canceled = true;
      },
    });
    const reader = watchdogPipe(upstream, 45_000).getReader();
    await reader.cancel();
    expect(canceled).toBe(true);
  });
});

describe('resolveCursor', () => {
  it('Last-Event-ID 헤더가 쿼리보다 우선한다', () => {
    expect(resolveCursor('7', '3')).toBe(7);
  });

  it('헤더가 없으면 쿼리로 폴백한다', () => {
    expect(resolveCursor(null, '3')).toBe(3);
  });

  it('둘 다 없으면 0이다', () => {
    expect(resolveCursor(null, null)).toBe(0);
  });

  it('비수치는 0으로 강등한다', () => {
    expect(resolveCursor('abc', null)).toBe(0);
  });

  it('음수는 0으로 강등한다', () => {
    expect(resolveCursor('-5', null)).toBe(0);
  });

  it('소수는 0으로 강등한다', () => {
    expect(resolveCursor('1.5', null)).toBe(0);
  });
});
