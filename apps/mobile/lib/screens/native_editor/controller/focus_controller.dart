import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/editor_input_view.dart';

class EditorFocusController {
  EditorFocusController({required this.inputKey, required this.onFocusChanged, required this.onCommitComposing});

  final GlobalKey<EditorInputViewState> inputKey;
  final void Function(bool focused) onFocusChanged;
  final VoidCallback onCommitComposing;

  bool _isActive = false;
  bool get isActive => _isActive;

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
    inputKey.currentState?.activateInput();
  }

  void clearFocus() {
    if (!_isActive) {
      return;
    }
    onCommitComposing();
    _isActive = false;
    onFocusChanged(false);
    inputKey.currentState?.deactivateInput();
  }

  void dismissKeyboard() {
    onCommitComposing();
    inputKey.currentState?.deactivateInput();
  }

  void onKeyboardHidden() {
    if (_isActive) {
      _isActive = false;
      onFocusChanged(false);
    }
  }

  void updateCursor(double x, double y, double height) {
    inputKey.currentState?.updateCursor(x, y, height);
  }

  void resetInputContext() {
    inputKey.currentState?.resetInputContext();
  }
}
