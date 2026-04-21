import 'dart:typed_data';

import 'package:opentelemetry/api.dart' as otel_api;
import 'package:opentelemetry/sdk.dart' as otel_sdk;
import 'package:typie/env.dart';

late otel_api.Tracer tracer;
late otel_sdk.BatchSpanProcessor? _processor;

void setupOpenTelemetry() {
  final url = Uri.parse('${Env.otelExporterOtlpEndpoint}/v1/traces');

  final exporter = otel_sdk.CollectorExporter(url);
  _processor = otel_sdk.BatchSpanProcessor(exporter);

  final rate = double.parse(Env.otelSampleRate);
  final provider = otel_sdk.TracerProviderBase(
    resource: otel_sdk.Resource([otel_api.Attribute.fromString('service.name', 'mobile')]),
    processors: [_processor!],
    sampler: TraceIdRatioBasedSampler(rate),
  );

  otel_api.registerGlobalTracerProvider(provider);

  tracer = otel_api.globalTracerProvider.getTracer('mobile');
}

class TraceIdRatioBasedSampler implements otel_sdk.Sampler {
  TraceIdRatioBasedSampler(double ratio)
    : _ratio = ratio.clamp(0.0, 1.0),
      _bound = (ratio.clamp(0.0, 1.0) * 0x7FFFFFFFFFFFFFFF).toInt();

  final double _ratio;
  final int _bound;

  @override
  otel_sdk.SamplingResult shouldSample(
    otel_api.Context context,
    otel_api.TraceId traceId,
    String spanName,
    otel_api.SpanKind spanKind,
    List<otel_api.Attribute> spanAttributes,
    List<otel_api.SpanLink> spanLinks,
  ) {
    if (_ratio >= 1.0) {
      return otel_sdk.SamplingResult(otel_sdk.Decision.recordAndSample, spanAttributes, otel_api.TraceState.empty());
    }
    if (_ratio <= 0.0) {
      return otel_sdk.SamplingResult(otel_sdk.Decision.drop, const [], otel_api.TraceState.empty());
    }

    final bytes = traceId.get();
    final lower = ByteData.sublistView(Uint8List.fromList(bytes.sublist(8))).getUint64(0) & 0x7FFFFFFFFFFFFFFF;

    final decision = lower < _bound ? otel_sdk.Decision.recordAndSample : otel_sdk.Decision.drop;

    return otel_sdk.SamplingResult(
      decision,
      decision == otel_sdk.Decision.drop ? const [] : spanAttributes,
      otel_api.TraceState.empty(),
    );
  }

  @override
  String get description => 'TraceIdRatioBasedSampler{ratio=$_ratio}';
}
