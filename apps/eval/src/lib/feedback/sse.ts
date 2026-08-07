export type SseEvent = { id: number | null; event: string; data: string };

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
