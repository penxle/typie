import { OTLPMetricExporter } from '@opentelemetry/exporter-metrics-otlp-grpc';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-grpc';
import { resourceFromAttributes } from '@opentelemetry/resources';
import { PeriodicExportingMetricReader } from '@opentelemetry/sdk-metrics';
import { NodeSDK } from '@opentelemetry/sdk-node';
import { AlwaysOnSampler } from '@opentelemetry/sdk-trace-node';
import { ATTR_SERVICE_NAME } from '@opentelemetry/semantic-conventions';
import { ATTR_DEPLOYMENT_ENVIRONMENT_NAME } from '@opentelemetry/semantic-conventions/incubating';
import * as Sentry from '@sentry/bun';
import { dev, env, stack } from '@/env';

Sentry.init({
  enabled: !dev,
  dsn: env.SENTRY_DSN,
  environment: stack,
  tracesSampleRate: 0.1,
  tracePropagationTargets: [],
  sendDefaultPii: true,
});

const sdk = new NodeSDK({
  sampler: new AlwaysOnSampler(),
  traceExporter: new OTLPTraceExporter({
    url: env.OTEL_EXPORTER_OTLP_ENDPOINT,
  }),
  metricReaders: [
    new PeriodicExportingMetricReader({
      exporter: new OTLPMetricExporter({
        url: env.OTEL_EXPORTER_OTLP_ENDPOINT,
      }),
    }),
  ],
  resource: resourceFromAttributes({
    [ATTR_SERVICE_NAME]: 'api',
    [ATTR_DEPLOYMENT_ENVIRONMENT_NAME]: stack,
  }),
});

sdk.start();
