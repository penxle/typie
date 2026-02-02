import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

Map<String, dynamic> _nav(String direction, bool extend) => {
  'type': 'navigate',
  'direction': direction,
  'extend': extend,
};

class EditorKeyboardHandler {
  EditorKeyboardHandler({required this.dispatch, required this.commitComposing});

  final void Function(Map<String, dynamic> message) dispatch;
  final void Function() commitComposing;

  bool handleKeyEvent(KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return false;
    }

    final message = _getActionFromKeyEvent(event);
    if (message == null) {
      return false;
    }

    if (message['type'] == 'navigate') {
      commitComposing();
    }
    dispatch(message);
    return true;
  }

  Map<String, dynamic>? _getActionFromKeyEvent(KeyEvent event) {
    final key = event.logicalKey;
    final shift = HardwareKeyboard.instance.isShiftPressed;
    final meta = HardwareKeyboard.instance.isMetaPressed;
    final ctrl = HardwareKeyboard.instance.isControlPressed;
    final alt = HardwareKeyboard.instance.isAltPressed;

    final wordModifier = defaultTargetPlatform == TargetPlatform.iOS ? alt : ctrl;
    final physical = event.physicalKey;

    if (key == LogicalKeyboardKey.arrowLeft || physical == PhysicalKeyboardKey.arrowLeft) {
      if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
        return _nav('lineStart', shift);
      } else if (wordModifier) {
        return _nav('wordLeft', shift);
      } else {
        return _nav('left', shift);
      }
    } else if (key == LogicalKeyboardKey.arrowRight || physical == PhysicalKeyboardKey.arrowRight) {
      if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
        return _nav('lineEnd', shift);
      } else if (wordModifier) {
        return _nav('wordRight', shift);
      } else {
        return _nav('right', shift);
      }
    } else if (key == LogicalKeyboardKey.arrowUp || physical == PhysicalKeyboardKey.arrowUp) {
      if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
        return _nav('documentStart', shift);
      } else if (alt) {
        return _nav('sentenceUp', shift);
      } else {
        return _nav('up', shift);
      }
    } else if (key == LogicalKeyboardKey.arrowDown || physical == PhysicalKeyboardKey.arrowDown) {
      if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
        return _nav('documentEnd', shift);
      } else if (alt) {
        return _nav('sentenceDown', shift);
      } else {
        return _nav('down', shift);
      }
    } else if (key == LogicalKeyboardKey.home || physical == PhysicalKeyboardKey.home) {
      if (ctrl) {
        return _nav('documentStart', shift);
      } else {
        return _nav('lineStart', shift);
      }
    } else if (key == LogicalKeyboardKey.end || physical == PhysicalKeyboardKey.end) {
      if (ctrl) {
        return _nav('documentEnd', shift);
      } else {
        return _nav('lineEnd', shift);
      }
    } else if (key == LogicalKeyboardKey.pageUp || physical == PhysicalKeyboardKey.pageUp) {
      return _nav('pageUp', shift);
    } else if (key == LogicalKeyboardKey.pageDown || physical == PhysicalKeyboardKey.pageDown) {
      return _nav('pageDown', shift);
    } else if (key == LogicalKeyboardKey.delete || physical == PhysicalKeyboardKey.delete) {
      if (wordModifier) {
        return {'type': 'deleteWordForward'};
      } else {
        return {'type': 'deleteForward'};
      }
    } else if (key == LogicalKeyboardKey.escape || physical == PhysicalKeyboardKey.escape) {
      return {'type': 'escape'};
    }

    return null;
  }
}
