import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/input.dart';

class InputController {
  InputController({
    required this.inputKey,
    required this.dispatch,
    required this.onFocusChanged,
    required ValueGetter<BottomToolbarMode> getBottomToolbarMode,
  }) : _getBottomToolbarMode = getBottomToolbarMode;

  final GlobalKey<InputViewState> inputKey;
  final void Function(Map<String, dynamic>) dispatch;
  final void Function(bool focused) onFocusChanged;
  final ValueGetter<BottomToolbarMode> _getBottomToolbarMode;

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
    inputKey.currentState?.activateInput();
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
    _deleteStartTime = null;
    dispatch({'type': 'input', 'text': text});
  }

  void onDeleteBackward() {
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
  }

  void onSetMarkedText(String text) {
    isComposing = true;
    dispatch({'type': 'compositionUpdate', 'text': text});
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
  }

  void onPerformAction(String action) {
    if (action == 'newline') {
      dispatch({'type': 'insertNewline'});
    }
  }

  void onShortcut(String action) {
    final direction = switch (action) {
      'navigateLeft' => 'left',
      'navigateRight' => 'right',
      'navigateUp' => 'up',
      'navigateDown' => 'down',
      _ => null,
    };

    if (direction != null) {
      dispatch({'type': 'navigate', 'direction': direction, 'extend': false});
    } else {
      dispatch({'type': action});
    }
  }

  void onFocusLost() {
    if (_getBottomToolbarMode() != BottomToolbarMode.hidden) {
      return;
    }
    clearFocus();
  }

  void onReplaceBackward(int length, String text) {
    dispatch({'type': 'replaceBackward', 'length': length, 'text': text});
  }

  void onKeyboardHidden() {
    if (_isActive) {
      _isActive = false;
      onFocusChanged(false);
    }
  }
}
