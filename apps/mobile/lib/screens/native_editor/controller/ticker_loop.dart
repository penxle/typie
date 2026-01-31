import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/scheduler.dart';
import 'package:typie/screens/native_editor/handler/command_handler.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

class EditorTickerLoop {
  EditorTickerLoop({required this.controller, required this.tickerProvider, required this.getSize});

  final EditorController controller;
  final TickerProvider tickerProvider;
  final (double, double) Function() getSize;

  Ticker? _ticker;
  (double, double, double)? _lastSize;

  void start() {
    _ticker = tickerProvider.createTicker(_onTick);
    unawaited(_ticker!.start());
  }

  void _onTick(Duration elapsed) {
    final editor = controller.editor;
    if (editor.isDisposed) {
      return;
    }

    final scaleFactor = ui.PlatformDispatcher.instance.views.first.devicePixelRatio;
    final (width, height) = getSize();
    final currentSize = (width, height, scaleFactor);

    if (_lastSize != currentSize) {
      _lastSize = currentSize;
      editor.dispatch({'type': 'resize', 'width': width, 'height': height, 'scaleFactor': scaleFactor});
    }

    final cmds = editor.tick();
    EditorCommandHandler.handleCommands(controller, cmds);

    unawaited(
      SchedulerBinding.instance.scheduleTask(() {
        if (!editor.isDisposed) {
          editor.flush();
        }
      }, Priority.idle),
    );
  }

  void dispose() {
    _ticker?.dispose();
    _ticker = null;
  }
}
