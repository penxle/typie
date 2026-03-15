import 'package:opentelemetry/api.dart' as otel_api;
import 'package:typie/otel.dart';

otel_api.Span? startFrameSpan({String? documentId}) {
  final span = tracer.startSpan('frame');
  if (!span.spanContext.isValid) {
    return null;
  }
  if (documentId != null) {
    span.setAttribute(otel_api.Attribute.fromString('document.id', documentId));
  }
  return span;
}
