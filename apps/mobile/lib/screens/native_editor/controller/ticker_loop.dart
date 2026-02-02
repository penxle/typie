import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/scheduler.dart';
import 'package:typie/screens/native_editor/handler/command_handler.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

class EditorTickerLoop {
  EditorTickerLoop({required this.controller, required this.tickerProvider, required this.getSize});

  static const _tickInterval = Duration(milliseconds: 16);

  final EditorController controller;
  final TickerProvider tickerProvider;
  final (double, double) Function() getSize;

  Ticker? _ticker;
  (double, double, double)? _lastSize;
  double _cachedScaleFactor = 0;
  Duration _lastTickTime = Duration.zero;

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

    final cmds = editor.tick();
    EditorCommandHandler.handleCommands(controller, cmds);

    if (!editor.isDisposed) {
      editor.flush();
    }
  }

  void dispose() {
    _ticker?.dispose();
    _ticker = null;
  }
}
