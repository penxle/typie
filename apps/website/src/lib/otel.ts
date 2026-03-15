import { SpanKind, trace } from '@opentelemetry/api';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-proto';
import { resourceFromAttributes } from '@opentelemetry/resources';
import { BatchSpanProcessor, TraceIdRatioBasedSampler } from '@opentelemetry/sdk-trace-base';
import { WebTracerProvider } from '@opentelemetry/sdk-trace-web';
import { ATTR_SERVICE_NAME } from '@opentelemetry/semantic-conventions';
import { ATTR_DEPLOYMENT_ENVIRONMENT_NAME } from '@opentelemetry/semantic-conventions/incubating';
import { match } from 'ts-pattern';
import { env } from '$env/dynamic/public';
import type { Tracer } from '@opentelemetry/api';
import type { ReadableSpan } from '@opentelemetry/sdk-trace-base';

let processor: BatchSpanProcessor;
export let tracer: Tracer;

export function setupOpenTelemetry() {
  const url = `${env.PUBLIC_OTEL_EXPORTER_OTLP_ENDPOINT}/v1/traces`;

  const exporter = new OTLPTraceExporter({ url });
  processor = new BatchSpanProcessor(exporter);

  const rate = Number.parseFloat(env.PUBLIC_OTEL_SAMPLE_RATE);
  const provider = new WebTracerProvider({
    resource: resourceFromAttributes({
      [ATTR_SERVICE_NAME]: 'website',
      [ATTR_DEPLOYMENT_ENVIRONMENT_NAME]: env.PUBLIC_ENVIRONMENT,
    }),
    sampler: new TraceIdRatioBasedSampler(rate),
    spanProcessors: [processor],
  });

  provider.register();

  tracer = trace.getTracer('website');
}

/* eslint-disable @typescript-eslint/no-explicit-any */
export function ingestSpans(spans: Record<string, any>[]) {
  for (const span of spans) {
    processor.onEnd(toReadableSpan(span));
  }
}

function toReadableSpan(span: Record<string, any>): ReadableSpan {
  return {
    name: span.name,
    spanContext: () => ({
      traceId: span.trace_id,
      spanId: span.span_id,
      traceFlags: 1,
      traceState: undefined,
    }),
    parentSpanContext: {
      traceId: span.trace_id,
      spanId: span.parent_span_id,
      traceFlags: 1,
      traceState: undefined,
    },
    kind: match(span.kind)
      .with(0, () => SpanKind.INTERNAL)
      .with(1, () => SpanKind.SERVER)
      .with(2, () => SpanKind.CLIENT)
      .with(3, () => SpanKind.PRODUCER)
      .with(4, () => SpanKind.CONSUMER)
      .run(),
    startTime: span.start_time,
    endTime: span.end_time,
    duration: span.duration,
    status: span.status,
    attributes: objectify(span.attributes),
    events: span.events.map((v: any) => ({ name: v.name, time: v.time, attributes: objectify(v.attributes) })),
    links: [],
    resource: resourceFromAttributes({ [ATTR_SERVICE_NAME]: 'editor' }),
    instrumentationScope: { name: 'editor' },
    ended: true,
    droppedAttributesCount: 0,
    droppedEventsCount: 0,
    droppedLinksCount: 0,
  };
}

const objectify = (value: any) => Object.fromEntries(value.map((v: any) => [v.key, v.value]));
/* eslint-enable @typescript-eslint/no-explicit-any */
