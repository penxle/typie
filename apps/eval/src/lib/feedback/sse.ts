import { PROJECTED_KINDS, projectFrame } from './frames.ts';

export type SseEvent = { id: number | null; event: string; data: string };

// EventSource는 named event만 리스너에 흘린다 — 릴레이가 사영해 내보내는 kind 전부다. 자식 run.*·invocation.*은
// 릴레이에서 떨어진다. 휘발 프레임 turn.delta는 로그에 남지 않아(id 없음) 커서·리듀서 밖 경로로 따로 받는다.
export const EVENT_NAMES: readonly string[] = PROJECTED_KINDS;

export const createSseParser = (): { push(chunk: string): SseEvent[] } => {
  let buffer = '';
  return {
    push(chunk) {
      buffer += chunk;
      const out: SseEvent[] = [];
      let idx: number;
      while ((idx = buffer.indexOf('\n\n')) >= 0) {
        const raw = buffer.slice(0, idx);
        buffer = buffer.slice(idx + 2);
        let id: number | null = null;
        let event = '';
        let data = '';
        for (const line of raw.split('\n')) {
          if (line.startsWith('id: ')) id = Number(line.slice(4));
          else if (line.startsWith('event: ')) event = line.slice(7);
          else if (line.startsWith('data: ')) data = line.slice(6);
        }
        if (event) out.push({ id, event, data });
      }
      return out;
    },
  };
};

// 로그 이벤트(id 있음)는 봉투를 사영하고, id 없는 프로토콜 프레임(sync·heartbeat·turn.delta)은 그대로 통과시킨다.
// 사영 밖 kind·깨진 봉투는 null — 릴레이·시드·저장이 같은 판정으로 떨군다.
export const projectEvent = (event: SseEvent): SseEvent | null => {
  if (event.id === null) return event;
  let parsed: unknown;
  try {
    parsed = JSON.parse(event.data);
  } catch {
    return null;
  }
  const frame = projectFrame(parsed);
  return frame === null ? null : { id: event.id, event: event.event, data: JSON.stringify(frame) };
};

export const serializeEvent = (event: SseEvent): string =>
  `${event.id === null ? '' : `id: ${event.id}\n`}event: ${event.event}\ndata: ${event.data}\n\n`;
