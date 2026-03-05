import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';

Map<String, dynamic> _nav(String direction, bool extend) => {
  'type': 'navigate',
  'direction': direction,
  'extend': extend,
};

class KeyboardHandler {
  KeyboardHandler({
    required this.dispatch,
    required this.reconcileInput,
    required this.scrollIntoView,
    required this.onShortcut,
  });

  final void Function(Map<String, dynamic> message) dispatch;
  final void Function() reconcileInput;
  final void Function({ScrollMode mode}) scrollIntoView;
  final void Function(String action) onShortcut;

  bool handleKeyEvent(KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return false;
    }

    if (_handleShortcut(event)) {
      return true;
    }

    final message = _getActionFromKeyEvent(event);
    if (message == null) {
      return false;
    }

    final type = message['type'] as String;
    if (type == 'navigate') {
      reconcileInput();
    }
    dispatch(message);

    if (type == 'navigate') {
      final extend = message['extend'] as bool? ?? false;
      scrollIntoView(mode: extend ? ScrollMode.auto : ScrollMode.typewriter);
    } else if (type == 'deleteForward' || type == 'deleteWordForward') {
      scrollIntoView(mode: ScrollMode.typewriter);
    } else {
      scrollIntoView();
    }

    return true;
  }

  bool _handleShortcut(KeyEvent event) {
    final key = event.logicalKey;
    final shift = HardwareKeyboard.instance.isShiftPressed;
    final meta = HardwareKeyboard.instance.isMetaPressed;
    final alt = HardwareKeyboard.instance.isAltPressed;

    if (!meta && !alt) {
      if (key == LogicalKeyboardKey.tab) {
        onShortcut(shift ? 'outdent' : 'indent');
        return true;
      }
      if (shift && key == LogicalKeyboardKey.enter) {
        onShortcut('insertHardBreak');
        return true;
      }
      return false;
    }

    if (meta && !alt) {
      if (key == LogicalKeyboardKey.keyB && !shift) {
        onShortcut('toggleBold');
        return true;
      } else if (key == LogicalKeyboardKey.keyI && !shift) {
        onShortcut('toggleItalic');
        return true;
      } else if (key == LogicalKeyboardKey.keyU && !shift) {
        onShortcut('toggleUnderline');
        return true;
      } else if (key == LogicalKeyboardKey.keyS && shift) {
        onShortcut('toggleStrikethrough');
        return true;
      } else if (key == LogicalKeyboardKey.backslash && !shift) {
        onShortcut('clearFormatting');
        return true;
      } else if (key == LogicalKeyboardKey.keyZ) {
        onShortcut(shift ? 'redo' : 'undo');
        return true;
      } else if (key == LogicalKeyboardKey.keyC && !shift) {
        onShortcut('copy');
        return true;
      } else if (key == LogicalKeyboardKey.keyX && !shift) {
        onShortcut('cut');
        return true;
      } else if (key == LogicalKeyboardKey.keyV && !shift) {
        onShortcut('paste');
        return true;
      } else if (key == LogicalKeyboardKey.keyA && !shift) {
        onShortcut('selectAll');
        return true;
      } else if (key == LogicalKeyboardKey.enter && !shift) {
        onShortcut('insertPageBreak');
        return true;
      } else if (key == LogicalKeyboardKey.backspace && !shift) {
        onShortcut('deleteToLineStart');
        return true;
      }
    }

    if (alt && !meta) {
      if (key == LogicalKeyboardKey.backspace && !shift) {
        onShortcut('deleteWordBackward');
        return true;
      }
    }

    return false;
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
      if (meta) {
        return _nav('lineStart', shift);
      } else if (wordModifier) {
        return _nav('wordLeft', shift);
      } else {
        return _nav('left', shift);
      }
    } else if (key == LogicalKeyboardKey.arrowRight || physical == PhysicalKeyboardKey.arrowRight) {
      if (meta) {
        return _nav('lineEnd', shift);
      } else if (wordModifier) {
        return _nav('wordRight', shift);
      } else {
        return _nav('right', shift);
      }
    } else if (key == LogicalKeyboardKey.arrowUp || physical == PhysicalKeyboardKey.arrowUp) {
      if (meta) {
        return _nav('documentStart', shift);
      } else if (alt) {
        return _nav('sentenceUp', shift);
      } else {
        return _nav('up', shift);
      }
    } else if (key == LogicalKeyboardKey.arrowDown || physical == PhysicalKeyboardKey.arrowDown) {
      if (meta) {
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
