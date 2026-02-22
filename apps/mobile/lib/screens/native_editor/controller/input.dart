import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/input.dart';

class InputController {
  InputController({
    required this.inputKey,
    required this.dispatch,
    required this.editor,
    required this.onFocusChanged,
    required this.scrollIntoView,
    required ValueGetter<BottomToolbarMode> getBottomToolbarMode,
    this.onInputAttempt,
  }) : _getBottomToolbarMode = getBottomToolbarMode;

  final GlobalKey<InputViewState> inputKey;
  final void Function(Map<String, dynamic>) dispatch;
  final NativeEditor editor;
  final void Function(bool focused) onFocusChanged;
  final void Function({ScrollMode mode}) scrollIntoView;
  final ValueGetter<BottomToolbarMode> _getBottomToolbarMode;
  final VoidCallback? onInputAttempt;

  bool _isActive = false;
  bool get isActive => _isActive;
  bool isComposing = false;
  bool _pendingFocus = false;
  bool _inputReady = false;

  DateTime? _deleteStartTime;
  DateTime? _lastDeleteSignal;

  void onInputReady() {
    _inputReady = true;
    if (_pendingFocus) {
      _pendingFocus = false;
      inputKey.currentState?.activateInput();
    }
  }

  void openInput() {
    if (!_isActive) {
      _isActive = true;
      onFocusChanged(true);
    }
    if (_inputReady) {
      inputKey.currentState?.activateInput();
    } else {
      _pendingFocus = true;
    }
  }

  void requestFocus() {
    _isActive = true;
    onFocusChanged(true);
    if (_inputReady) {
      inputKey.currentState?.activateInput();
    } else {
      _pendingFocus = true;
    }
  }

  void clearFocus() {
    if (!_isActive) {
      return;
    }
    _pendingFocus = false;
    commitComposing();
    _isActive = false;
    onFocusChanged(false);
    inputKey.currentState?.deactivateInput();
  }

  void dismissKeyboard() {
    commitComposing();
    inputKey.currentState?.deactivateInput();
  }

  void updateCursor(double x, double y, double height, [List<double>? precedingCharWidths]) {
    inputKey.currentState?.updateCursor(x, y, height, precedingCharWidths);
  }

  void commitComposing() {
    if (isComposing) {
      isComposing = false;
      dispatch({'type': 'commitPreedit'});
    }
    inputKey.currentState?.resetInputContext();
  }

  void onInsertText(String text) {
    onInputAttempt?.call();
    _deleteStartTime = null;
    dispatch({'type': 'input', 'text': text});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void onDeleteBackward() {
    onInputAttempt?.call();
    final now = DateTime.now();
    final lastSignal = _lastDeleteSignal;
    _lastDeleteSignal = now;

    final isRepeating = lastSignal != null && now.difference(lastSignal).inMilliseconds < 500;

    if (!isRepeating) {
      _deleteStartTime = null;
    }

    _deleteStartTime ??= now;
    final duration = now.difference(_deleteStartTime!).inMilliseconds / 1000.0;

    if (duration > 3.0) {
      dispatch({'type': 'deleteSentenceBackward'});
    } else if (duration > 1.5) {
      dispatch({'type': 'deleteWordBackward'});
    } else {
      dispatch({'type': 'deleteBackward'});
    }
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void onSetMarkedText(String text) {
    onInputAttempt?.call();
    isComposing = true;
    dispatch({'type': 'compositionUpdate', 'text': text});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void onUnmarkText() {
    if (isComposing) {
      isComposing = false;
      dispatch({'type': 'commitPreedit'});
    }
  }

  void onCancelMarkedText() {
    isComposing = false;
    dispatch({'type': 'compositionEnd'});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void onPerformAction(String action) {
    onInputAttempt?.call();
    if (action == 'newline') {
      dispatch({'type': 'insertNewline'});
      scrollIntoView(mode: ScrollMode.typewriter);
    }
  }

  void onShortcut(String action) {
    onInputAttempt?.call();
    final direction = switch (action) {
      'navigateLeft' => 'left',
      'navigateRight' => 'right',
      'navigateUp' => 'up',
      'navigateDown' => 'down',
      _ => null,
    };

    if (direction != null) {
      dispatch({'type': 'navigate', 'direction': direction, 'extend': false});
      scrollIntoView(mode: ScrollMode.typewriter);
    } else if (action == 'copy') {
      unawaited(EditorClipboard().copy(editor));
    } else if (action == 'cut') {
      unawaited(
        EditorClipboard().cut(editor, (msg) {
          dispatch(msg);
          scrollIntoView(mode: ScrollMode.typewriter);
        }),
      );
    } else if (action == 'paste') {
      if (onPasteHandler != null) {
        unawaited(onPasteHandler!());
      } else {
        unawaited(
          EditorClipboard().getPastePayload().then((payload) {
            if (payload != null) {
              dispatch(payload);
              scrollIntoView(mode: ScrollMode.typewriter);
            }
          }),
        );
      }
    } else if (action == 'toggleItalic') {
      dispatch({
        'type': 'toggleStyle',
        'style': {'type': 'italic'},
      });
      scrollIntoView();
    } else if (action == 'toggleUnderline') {
      dispatch({
        'type': 'toggleStyle',
        'style': {'type': 'underline'},
      });
      scrollIntoView();
    } else if (action == 'toggleStrikethrough') {
      dispatch({
        'type': 'toggleStyle',
        'style': {'type': 'strikethrough'},
      });
      scrollIntoView();
    } else if (action == 'selectAll') {
      dispatch({'type': action});
    } else {
      dispatch({'type': action});
      scrollIntoView();
    }
  }

  Future<void> Function()? onPasteHandler;

  VoidCallback? floatingCursorBeginHandler;
  void Function(double dx, double dy)? floatingCursorUpdateHandler;
  VoidCallback? floatingCursorEndHandler;

  void onFloatingCursorBegin() {
    commitComposing();
    floatingCursorBeginHandler?.call();
  }

  void onFloatingCursorUpdate(double dx, double dy) {
    floatingCursorUpdateHandler?.call(dx, dy);
  }

  void onFloatingCursorEnd() {
    floatingCursorEndHandler?.call();
  }

  void onFocusLost() {
    if (_getBottomToolbarMode() != BottomToolbarMode.hidden) {
      return;
    }
    clearFocus();
  }

  void onReplaceBackward(int length, String text) {
    onInputAttempt?.call();
    dispatch({'type': 'replaceBackward', 'length': length, 'text': text});
    scrollIntoView(mode: ScrollMode.typewriter);
  }

  void onKeyboardHidden() {
    if (_isActive) {
      _isActive = false;
      onFocusChanged(false);
    }
  }
}
