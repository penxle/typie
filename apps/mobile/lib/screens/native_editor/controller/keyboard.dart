import 'package:flutter/services.dart';
import 'package:typie/screens/native_editor/state/scroll_mode.dart';

Map<String, dynamic> _nav(String direction, bool extend) => {
  'type': 'navigate',
  'direction': direction,
  'extend': extend,
};

bool _matchesPrintableShortcut(
  KeyEvent event, {
  required LogicalKeyboardKey logical,
  required PhysicalKeyboardKey physical,
}) {
  if (event.logicalKey == logical) {
    return true;
  }

  if (event.physicalKey != physical) {
    return false;
  }

  return _shouldUsePhysicalShortcutFallback(event.logicalKey);
}

bool _shouldUsePhysicalShortcutFallback(LogicalKeyboardKey logicalKey) {
  final label = logicalKey.keyLabel;
  if (label.isEmpty) {
    return true;
  }

  for (final codePoint in label.runes) {
    if (codePoint > 0x7f) {
      // non-ascii
      return true;
    }
  }

  return false;
}

class _Modifiers {
  const _Modifiers({required this.shift, required this.meta, required this.ctrl, required this.alt});

  factory _Modifiers.current() {
    final keyboard = HardwareKeyboard.instance;
    return _Modifiers(
      shift: keyboard.isShiftPressed,
      meta: keyboard.isMetaPressed,
      ctrl: keyboard.isControlPressed,
      alt: keyboard.isAltPressed,
    );
  }

  final bool shift;
  final bool meta;
  final bool ctrl;
  final bool alt;

  bool get shortcut => meta || ctrl;
  bool get word => alt || ctrl;
}

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
      scrollIntoView(mode: ScrollMode.typewriter);
    } else if (type == 'deleteForward' || type == 'deleteWordForward') {
      scrollIntoView(mode: ScrollMode.typewriter);
    } else {
      scrollIntoView();
    }

    return true;
  }

  bool _handleShortcut(KeyEvent event) {
    final key = event.logicalKey;
    final modifiers = _Modifiers.current();

    if (!modifiers.shortcut && !modifiers.alt) {
      if (key == LogicalKeyboardKey.tab) {
        onShortcut(modifiers.shift ? 'outdent' : 'indent');
        return true;
      }
      if (modifiers.shift && key == LogicalKeyboardKey.enter) {
        onShortcut('insertHardBreak');
        return true;
      }
      return false;
    }

    if (modifiers.shortcut && !modifiers.alt) {
      // IMEs can remap printable logical keys while preserving the physical key.
      if (_matchesPrintableShortcut(event, logical: LogicalKeyboardKey.keyB, physical: PhysicalKeyboardKey.keyB) &&
          !modifiers.shift) {
        onShortcut('toggleBold');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.keyI,
            physical: PhysicalKeyboardKey.keyI,
          ) &&
          !modifiers.shift) {
        onShortcut('toggleItalic');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.keyU,
            physical: PhysicalKeyboardKey.keyU,
          ) &&
          !modifiers.shift) {
        onShortcut('toggleUnderline');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.keyS,
            physical: PhysicalKeyboardKey.keyS,
          ) &&
          modifiers.shift) {
        onShortcut('toggleStrikethrough');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.backslash,
            physical: PhysicalKeyboardKey.backslash,
          ) &&
          !modifiers.shift) {
        onShortcut('clearFormatting');
        return true;
      } else if (_matchesPrintableShortcut(
        event,
        logical: LogicalKeyboardKey.keyZ,
        physical: PhysicalKeyboardKey.keyZ,
      )) {
        onShortcut(modifiers.shift ? 'redo' : 'undo');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.keyC,
            physical: PhysicalKeyboardKey.keyC,
          ) &&
          !modifiers.shift) {
        onShortcut('copy');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.keyX,
            physical: PhysicalKeyboardKey.keyX,
          ) &&
          !modifiers.shift) {
        onShortcut('cut');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.keyV,
            physical: PhysicalKeyboardKey.keyV,
          ) &&
          !modifiers.shift) {
        onShortcut('paste');
        return true;
      } else if (_matchesPrintableShortcut(
            event,
            logical: LogicalKeyboardKey.keyA,
            physical: PhysicalKeyboardKey.keyA,
          ) &&
          !modifiers.shift) {
        onShortcut('selectAll');
        return true;
      } else if (key == LogicalKeyboardKey.enter && !modifiers.shift) {
        onShortcut('insertPageBreak');
        return true;
      } else if (key == LogicalKeyboardKey.backspace && modifiers.meta && !modifiers.shift) {
        onShortcut('deleteToLineStart');
        return true;
      }
    }

    if (modifiers.word && !modifiers.meta) {
      if (key == LogicalKeyboardKey.backspace && !modifiers.shift) {
        onShortcut('deleteWordBackward');
        return true;
      }
    }

    return false;
  }

  Map<String, dynamic>? _getActionFromKeyEvent(KeyEvent event) {
    final key = event.logicalKey;
    final modifiers = _Modifiers.current();
    final physical = event.physicalKey;

    if (key == LogicalKeyboardKey.arrowLeft || physical == PhysicalKeyboardKey.arrowLeft) {
      if (modifiers.meta) {
        return _nav('lineStart', modifiers.shift);
      } else if (modifiers.word) {
        return _nav('wordLeft', modifiers.shift);
      } else {
        return _nav('left', modifiers.shift);
      }
    } else if (key == LogicalKeyboardKey.arrowRight || physical == PhysicalKeyboardKey.arrowRight) {
      if (modifiers.meta) {
        return _nav('lineEnd', modifiers.shift);
      } else if (modifiers.word) {
        return _nav('wordRight', modifiers.shift);
      } else {
        return _nav('right', modifiers.shift);
      }
    } else if (key == LogicalKeyboardKey.arrowUp || physical == PhysicalKeyboardKey.arrowUp) {
      if (modifiers.meta) {
        return _nav('documentStart', modifiers.shift);
      } else if (modifiers.alt) {
        return _nav('sentenceUp', modifiers.shift);
      } else {
        return _nav('up', modifiers.shift);
      }
    } else if (key == LogicalKeyboardKey.arrowDown || physical == PhysicalKeyboardKey.arrowDown) {
      if (modifiers.meta) {
        return _nav('documentEnd', modifiers.shift);
      } else if (modifiers.alt) {
        return _nav('sentenceDown', modifiers.shift);
      } else {
        return _nav('down', modifiers.shift);
      }
    } else if (key == LogicalKeyboardKey.home || physical == PhysicalKeyboardKey.home) {
      if (modifiers.ctrl) {
        return _nav('documentStart', modifiers.shift);
      } else {
        return _nav('lineStart', modifiers.shift);
      }
    } else if (key == LogicalKeyboardKey.end || physical == PhysicalKeyboardKey.end) {
      if (modifiers.ctrl) {
        return _nav('documentEnd', modifiers.shift);
      } else {
        return _nav('lineEnd', modifiers.shift);
      }
    } else if (key == LogicalKeyboardKey.pageUp || physical == PhysicalKeyboardKey.pageUp) {
      return _nav('pageUp', modifiers.shift);
    } else if (key == LogicalKeyboardKey.pageDown || physical == PhysicalKeyboardKey.pageDown) {
      return _nav('pageDown', modifiers.shift);
    } else if (key == LogicalKeyboardKey.delete || physical == PhysicalKeyboardKey.delete) {
      if (modifiers.word && !modifiers.meta) {
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
