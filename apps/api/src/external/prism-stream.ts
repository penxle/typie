import { StreamFrameSchema } from '@typie/prism';
import { createParser } from 'eventsource-parser';
import type { StreamFrame } from '@typie/prism';

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
}): Promise<PumpOutcome> => {
  const reader = opts.stream.getReader();
  const decoder = new TextDecoder();
  const parser = createSseParser();
  let onAbort: (() => void) | null = null;
  const aborted = new Promise<'aborted'>((resolve) => {
    if (opts.signal.aborted) {
      resolve('aborted');
      return;
    }

    onAbort = () => resolve('aborted');
    opts.signal.addEventListener('abort', onAbort, { once: true });
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
        await opts.onFrame(parseStreamFrame(event));
      }
    }
  } finally {
    if (onAbort !== null) opts.signal.removeEventListener('abort', onAbort);
    await reader.cancel().catch(() => null);
  }
};
