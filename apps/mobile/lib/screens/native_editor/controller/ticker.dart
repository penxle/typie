import 'dart:async';

import 'package:flutter/scheduler.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/handler/command.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

class EditorTicker {
  EditorTicker({required this.getController, required this.tickerProvider});

  final EditorController Function() getController;
  final TickerProvider tickerProvider;

  Ticker? _ticker;
  bool _flushPending = false;
  bool _inputFastPathQueued = false;
  List<Completer<void>> _settledCompleters = [];

  static bool _isInputFastPathMessage(Map<String, dynamic> message) {
    final type = message['type'];
    if (type is! String) {
      return false;
    }

    switch (type) {
      case 'input':
      case 'replaceBackward':
      case 'deleteBackward':
      case 'deleteWordBackward':
      case 'deleteSentenceBackward':
      case 'compositionStart':
      case 'compositionUpdate':
      case 'compositionEnd':
      case 'commitPreedit':
        return true;
      default:
        return false;
    }
  }

  Future<void> settled() {
    final completer = Completer<void>();
    _settledCompleters.add(completer);
    return completer.future;
  }

  void start() {
    _ticker ??= tickerProvider.createTicker(_onTick);
    final controller = getController();
    controller.editor.onWakeUp = _ensureActive;
    controller.onDispatched = onDispatchedMessage;
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

    _runTickCycle(controller, editor);
  }

  void onDispatchedMessage(Map<String, dynamic> message) {
    if (!_isInputFastPathMessage(message)) {
      return;
    }
    _wakeUpInputFastPath();
  }

  void _wakeUpInputFastPath() {
    if (_inputFastPathQueued) {
      return;
    }
    _inputFastPathQueued = true;

    scheduleMicrotask(() {
      _inputFastPathQueued = false;
      final controller = getController();
      final editor = controller.editor;
      if (editor.isDisposed || !editor.awake) {
        return;
      }
      _runTickCycle(controller, editor);
      if (editor.awake) {
        _ensureActive();
      }
    });
  }

  void _runTickCycle(EditorController controller, NativeEditor editor) {
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
    getController().onDispatched = null;
    stop();
    _ticker?.dispose();
  }
}
