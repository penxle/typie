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

// prism은 lastEventId로 음수·비정수를 400으로 거부한다. 재접속 커서는 브라우저가 준 값이라
// 신뢰 대상이 아니므로, 거부당할 값은 보내지 않고 처음부터 다시 받는다.
export const resolveCursor = (headerValue: string | null, queryValue: string | null): number => {
  const cursor = Number(headerValue ?? queryValue ?? '0');
  return Number.isSafeInteger(cursor) && cursor >= 0 ? cursor : 0;
};
