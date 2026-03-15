import { ingestSpans, tracer } from '$lib/otel';

export function startFrameSpan(documentId?: string) {
  const span = tracer.startSpan('frame');
  if (!span.isRecording()) return null;
  if (documentId) span.setAttribute('document.id', documentId);
  return span;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function drainTraces(fn: () => any) {
  const traces = fn();
  if (!traces || traces.length === 0) return;

  for (const spans of traces) {
    ingestSpans(spans);
  }
}
