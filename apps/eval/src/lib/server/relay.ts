import { createSseParser, projectEvent, serializeEvent } from '../feedback/sse.ts';

// half-open 방어는 릴레이가 소유한다 — 브라우저 EventSource는 comment 하트비트를 관측할 수 없어
// 무수신 판정이 불가능하다. 여기서 닫아 주면 EventSource가 Last-Event-ID를 들고 자동 재접속한다.
export const watchdogPipe = (upstream: ReadableStream<Uint8Array>, idleMs: number): ReadableStream<Uint8Array> => {
  const reader = upstream.getReader();
  return new ReadableStream<Uint8Array>({
    async pull(controller) {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const idle = new Promise<'idle'>((resolve) => {
        timer = setTimeout(() => resolve('idle'), idleMs);
      });
      const outcome = await Promise.race([reader.read().catch(() => ({ done: true as const, value: undefined })), idle]).finally(() =>
        clearTimeout(timer),
      );
      if (outcome === 'idle' || outcome.done || outcome.value === undefined) {
        // eslint-disable-next-line @typescript-eslint/no-empty-function -- 이미 닫는 중이라 취소 실패는 삼킨다
        await reader.cancel().catch(() => {});
        controller.close();
        return;
      }
      controller.enqueue(outcome.value);
    },
    cancel() {
      // eslint-disable-next-line @typescript-eslint/no-empty-function -- 다운스트림이 이미 떠났으므로 취소 실패는 삼킨다
      return reader.cancel().catch(() => {});
    },
  });
};

// 프레임 사영 — 로그 이벤트(id 있음)는 봉투를 화면 형태로 줄이고(frames.ts — 도구 input·output 원문·턴 raw가 MB 단위로
// 실리는 전문 계약을 브라우저에 그대로 흘리지 않는다), 사영 밖 kind는 떨군다. id 없는 프로토콜 프레임(sync·heartbeat·
// turn.delta)은 그대로 지나간다. 떨어진 프레임은 Last-Event-ID에 공백을 남기지만 커서는 "이 seq까지"라 무해하다(§2.1).
// 워치독 뒤에 둔다 — 무수신 판정은 업스트림 바이트 기준이어야 한다(떨어지는 프레임만 이어지는 구간을 침묵으로 오판하지 않는다).
export const projectPipe = (upstream: ReadableStream<Uint8Array>): ReadableStream<Uint8Array> => {
  const reader = upstream.getReader();
  const parser = createSseParser();
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  return new ReadableStream<Uint8Array>({
    // 떨어지는 프레임만 이어지는 청크는 출력이 없다 — 출력이 생기거나 EOF까지 업스트림을 계속 읽는다(pull 1회 = 출력 1회).
    async pull(controller) {
      for (;;) {
        const { done, value } = await reader.read();
        if (done || value === undefined) {
          controller.close();
          return;
        }
        // eslint-disable-next-line unicorn/no-return-array-push -- the parser's push() returns parsed events
        const frames = parser.push(decoder.decode(value, { stream: true }));
        const out = frames.flatMap((event) => {
          const projected = projectEvent(event);
          return projected === null ? [] : [serializeEvent(projected)];
        });
        if (out.length > 0) {
          controller.enqueue(encoder.encode(out.join('')));
          return;
        }
      }
    },
    cancel() {
      // eslint-disable-next-line @typescript-eslint/no-empty-function -- 다운스트림이 이미 떠났으므로 취소 실패는 삼킨다
      return reader.cancel().catch(() => {});
    },
  });
};

// prism은 lastEventId로 음수·비정수를 400으로 거부한다. 재접속 커서는 브라우저가 준 값이라
// 신뢰 대상이 아니므로, 거부당할 값은 보내지 않고 처음부터 다시 받는다.
export const resolveCursor = (headerValue: string | null, queryValue: string | null): number => {
  const cursor = Number(headerValue ?? queryValue ?? '0');
  return Number.isSafeInteger(cursor) && cursor >= 0 ? cursor : 0;
};
