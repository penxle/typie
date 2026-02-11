import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/scheduler.dart';
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/handler/command.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

class TickerLoop {
  TickerLoop({required this.controller, required this.tickerProvider, required this.getSize});

  static const _tickInterval = Duration(milliseconds: 16);

  final EditorController controller;
  final TickerProvider tickerProvider;
  final (double, double) Function() getSize;

  Ticker? _ticker;
  (double, double, double)? _lastSize;
  double _cachedScaleFactor = 0;
  Duration _lastTickTime = Duration.zero;
  bool _flushPending = false;

  void start() {
    _cachedScaleFactor = ui.PlatformDispatcher.instance.views.first.devicePixelRatio;
    _ticker = tickerProvider.createTicker(_onTick);
    unawaited(_ticker!.start());
  }

  void _onTick(Duration elapsed) {
    if (elapsed - _lastTickTime < _tickInterval) {
      return;
    }
    _lastTickTime = elapsed;

    final editor = controller.editor;
    if (editor.isDisposed) {
      return;
    }

    final (width, height) = getSize();
    final currentSize = (width, height, _cachedScaleFactor);

    if (_lastSize != currentSize) {
      _lastSize = currentSize;
      editor.dispatch({'type': 'resize', 'width': width, 'height': height, 'scaleFactor': _cachedScaleFactor});
    }

    editor.tick();

    final slatePtr = editor.getSlatePtr();
    final slateLen = editor.getSlateLen();
    final slabPtr = editor.getSlabPtr();
    final slabLen = editor.getSlabLen();

    final reader = SlateReader(slatePtr, slateLen, slabPtr, slabLen);
    CommandHandler.handleSlate(controller, reader);

    if (!editor.isDisposed) {
      controller.notifyTick();

      if (!_flushPending) {
        _flushPending = true;
        unawaited(
          SchedulerBinding.instance.scheduleTask(() {
            _flushPending = false;
            if (editor.isDisposed) {
              return;
            }
            editor.flush();
          }, Priority.idle),
        );
      }
    }
  }

  void dispose() {
    _ticker?.dispose();
    _ticker = null;
  }
}
