import 'dart:async';

import 'package:flutter/scheduler.dart';
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/handler/command.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

class TickerLoop {
  TickerLoop({required this.getController, required this.tickerProvider});

  static const _tickInterval = Duration(milliseconds: 16);

  final EditorController Function() getController;
  final TickerProvider tickerProvider;

  Ticker? _ticker;
  Duration _lastTickTime = Duration.zero;
  bool _flushPending = false;

  void start() {
    _ticker ??= tickerProvider.createTicker(_onTick);
    if (_ticker!.isActive) {
      return;
    }
    _lastTickTime = Duration.zero;
    unawaited(_ticker!.start());
  }

  void stop() {
    _ticker?.stop();
  }

  void _onTick(Duration elapsed) {
    if (elapsed - _lastTickTime < _tickInterval) {
      return;
    }
    _lastTickTime = elapsed;

    final controller = getController();
    final editor = controller.editor;
    if (editor.isDisposed || !editor.needsTick) {
      return;
    }

    editor
      ..tick()
      ..resetNeedsTick();

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
    stop();
  }
}
