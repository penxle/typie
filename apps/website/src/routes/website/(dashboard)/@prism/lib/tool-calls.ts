import type { TranscriptMessage } from './conversation.ts';

export type ToolRow = { label: string; count: number };

export const collapseRows = (labels: string[]): ToolRow[] =>
  labels.reduce<ToolRow[]>((rows, label) => {
    const last = rows.at(-1);
    return last !== undefined && last.label === label
      ? [...rows.slice(0, -1), { label, count: last.count + 1 }]
      : [...rows, { label, count: 1 }];
  }, []);

export type TranscriptEntry = TranscriptMessage | { role: 'tool-calls'; key: string; count: number; rows: ToolRow[] };

export const foldToolCalls = (
  messages: TranscriptMessage[],
  foldable: (message: TranscriptMessage) => boolean,
  labelOf: (message: TranscriptMessage) => string | null,
): TranscriptEntry[] => {
  const entries: TranscriptEntry[] = [];
  let labels: string[] = [];
  let key = '';

  const flush = () => {
    if (labels.length === 0) return;
    entries.push({ role: 'tool-calls', key: `tools:${key}`, count: labels.length, rows: collapseRows(labels) });
    labels = [];
  };

  for (const message of messages) {
    if (foldable(message)) {
      const label = labelOf(message);
      if (label === null) continue;
      if (labels.length === 0) key = message.key;
      labels.push(label);
      continue;
    }
    flush();
    entries.push(message);
  }
  flush();
  return entries;
};
