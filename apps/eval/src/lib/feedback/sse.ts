export type SseEvent = { id: number | null; event: string; data: string };

// EventSource는 named event만 리스너에 흘린다 — 세션 화면이 구독하는 로그 이벤트 전부다. 자식 run.*·invocation.*은
// 리듀서에 소비처가 없어 받지 않는다(Last-Event-ID는 리스너와 무관하게 갱신된다). 휘발 프레임 turn.delta는 로그에
// 남지 않아(id 없음) 커서·리듀서 밖 경로로 따로 받는다. turn.started는 리듀서가 아니라 흐르는 턴 조각이 쓴다.
export const EVENT_NAMES = [
  'workflow.started',
  'step.started',
  'step.completed',
  'turn.started',
  'turn.completed',
  'tool.requested',
  'tool.called',
  'workflow.completed',
  'workflow.failed',
  'workflow.canceled',
];

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
