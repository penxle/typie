import 'dart:async';

import 'package:flutter/scheduler.dart';
import 'package:opentelemetry/api.dart' as otel_api;
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/controller/tracing.dart';
import 'package:typie/screens/native_editor/handler/command.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

/// How often to drain accumulated Rust perf spans.
const _drainInterval = Duration(seconds: 5);

class EditorTicker {
  EditorTicker({required this.getController, required this.tickerProvider});

  final EditorController Function() getController;
  final TickerProvider tickerProvider;

  Ticker? _ticker;
  bool _flushPending = false;
  List<Completer<void>> _settledCompleters = [];

  otel_api.Span? _span;
  otel_api.Span? get span => _span;

  Timer? _drainTimer;

  Future<void> settled() {
    final completer = Completer<void>();
    _settledCompleters.add(completer);
    return completer.future;
  }

  void start() {
    _ticker ??= tickerProvider.createTicker(_onTick);
    getController().editor.onWakeUp = _ensureActive;
    _drainTimer ??= Timer.periodic(_drainInterval, (_) => _drainTraces());
    _ensureActive();
  }

  void stop() {
    _ticker?.stop();
  }

  void _ensureActive() {
    final ticker = _ticker;
    if (ticker == null || ticker.isActive) {
      return;
    }
    unawaited(ticker.start());
  }

  void _drainTraces() {
    final controller = getController();
    final editor = controller.editor;
    if (editor.isDisposed) {
      return;
    }

    // XXX: once Dart OTel SDK supports injecting external spans,
    // parse and forward drained Rust spans to the OTel pipeline.
    editor.drainTraces();
  }

  void _onTick(Duration elapsed) {
    final controller = getController();
    final editor = controller.editor;
    if (editor.isDisposed || !editor.awake) {
      stop();
      return;
    }

    if (_span != null) {
      _span!.end();
      _span = null;
    }
    _span = startFrameSpan(documentId: controller.documentId);

    if (_span != null) {
      final ctx = _span!.spanContext;
      editor.setTracing(ctx.traceId.toString(), ctx.spanId.toString());
    }

    editor
      ..tick()
      ..resetAwake();

    if (_span != null) {
      editor.clearTracing();
    }

    final slatePtr = editor.getSlatePtr();
    final slateLen = editor.getSlateLen();
    final slabPtr = editor.getSlabPtr();
    final slabLen = editor.getSlabLen();

    final reader = SlateReader(slatePtr, slateLen, slabPtr, slabLen);
    CommandHandler.handleSlate(controller, reader);

    if (!editor.isDisposed) {
      if (_settledCompleters.isNotEmpty) {
        final completers = _settledCompleters;
        _settledCompleters = [];
        for (final completer in completers) {
          completer.complete();
        }
      }

      if (!_flushPending) {
        _flushPending = true;
        SchedulerBinding.instance.addPostFrameCallback((_) {
          _flushPending = false;
          if (editor.isDisposed) {
            return;
          }
          editor.flush();
        });
      }
    }
  }

  void dispose() {
    stop();
    _drainTimer?.cancel();
    _drainTimer = null;
    _span?.end();
    _span = null;
    _drainTraces();
    _ticker?.dispose();
  }
}
