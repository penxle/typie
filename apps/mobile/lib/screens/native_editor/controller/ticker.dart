import 'dart:async';

import 'package:flutter/scheduler.dart';
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/handler/command.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

class EditorTicker {
  EditorTicker({required this.getController, required this.tickerProvider});

  final EditorController Function() getController;
  final TickerProvider tickerProvider;

  Ticker? _ticker;
  bool _flushPending = false;
  List<Completer<void>> _settledCompleters = [];

  Future<void> settled() {
    final completer = Completer<void>();
    _settledCompleters.add(completer);
    return completer.future;
  }

  void start() {
    _ticker ??= tickerProvider.createTicker(_onTick);
    getController().editor.onWakeUp = _ensureActive;
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

  void _onTick(Duration elapsed) {
    final controller = getController();
    final editor = controller.editor;
    if (editor.isDisposed || !editor.awake) {
      stop();
      return;
    }

    editor
      ..tick()
      ..resetAwake();

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
    _ticker?.dispose();
  }
}
