export const computeSegments = (
  text: string,
  anchors: { start: number; end: number; feedbackId: string }[],
): { text: string; feedbackIds: string[] }[] => {
  const valid = anchors.filter((a) => a.start >= 0 && a.end <= text.length && a.start < a.end);
  const bounds = new Set([0, text.length]);
  for (const anchor of valid) {
    bounds.add(anchor.start);
    bounds.add(anchor.end);
  }
  const sorted = [...bounds].toSorted((x, y) => x - y);

  const segments: { text: string; feedbackIds: string[] }[] = [];
  for (let i = 0; i < sorted.length - 1; i++) {
    const [start, end] = [sorted[i], sorted[i + 1]];
    const feedbackIds = valid.filter((a) => a.start <= start && end <= a.end).map((a) => a.feedbackId);
    segments.push({ text: text.slice(start, end), feedbackIds });
  }
  return segments;
};
