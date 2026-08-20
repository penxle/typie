import { StreamFrameSchema } from '@typie/prism';
import { createParser } from 'eventsource-parser';
import type { EventFrame, StreamFrame } from '@typie/prism';

export type SseEvent = { event: string; data: string };

export const createSseParser = (): { feed(chunk: string): SseEvent[] } => {
  let batch: SseEvent[] = [];

  const parser = createParser({
    onEvent(event) {
      if (event.event === undefined || event.event.length === 0) {
        return;
      }

      batch.push({ event: event.event, data: event.data });
    },
  });

  return {
    feed(chunk) {
      batch = [];
      parser.feed(chunk);
      return batch;
    },
  };
};

export const parseStreamFrame = (event: SseEvent): StreamFrame => StreamFrameSchema.parse(event);

export type PumpOutcome = 'closed' | 'idle' | 'aborted';

export const pumpSse = async (opts: {
  stream: ReadableStream<Uint8Array>;
  onFrame: (frame: StreamFrame) => void | Promise<void>;
  idleMs: number;
  signal: AbortSignal;
  stopAtSync?: boolean;
}): Promise<PumpOutcome> => {
  const reader = opts.stream.getReader();
  const decoder = new TextDecoder();
  const parser = createSseParser();
  const aborted = new Promise<'aborted'>((resolve) => {
    if (opts.signal.aborted) resolve('aborted');
    else opts.signal.addEventListener('abort', () => resolve('aborted'), { once: true });
  });

  try {
    for (;;) {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const idle = new Promise<'idle'>((resolve) => {
        timer = setTimeout(() => resolve('idle'), opts.idleMs);
      });
      const outcome = await Promise.race([reader.read().catch(() => ({ done: true as const, value: undefined })), idle, aborted]).finally(
        () => clearTimeout(timer),
      );
      if (outcome === 'idle' || outcome === 'aborted') return outcome;
      if (outcome.done) return 'closed';
      for (const event of parser.feed(decoder.decode(outcome.value, { stream: true }))) {
        const frame = parseStreamFrame(event);
        await opts.onFrame(frame);
        if (opts.stopAtSync && frame.type === 'sync') return 'closed';
      }
    }
  } finally {
    await reader.cancel().catch(() => null);
  }
};

export const readUntilSync = async (
  stream: ReadableStream<Uint8Array>,
  signal: AbortSignal,
  idleMs = 45_000,
): Promise<{ events: EventFrame[]; sync: number }> => {
  const events: EventFrame[] = [];
  let sync = -1;

  const outcome = await pumpSse({
    stream,
    idleMs,
    signal,
    stopAtSync: true,
    onFrame: (frame) => {
      if (frame.type === 'event') events.push(frame.event);
      else if (frame.type === 'sync') sync = frame.seq;
    },
  });

  if (sync < 0) throw new Error(`event replay ended without sync: ${outcome}`);

  return { events, sync };
};
