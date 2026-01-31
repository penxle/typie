import 'package:typie/screens/native_editor/handler/commands.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

typedef CommandExecutor = void Function(EditorController controller, Map<String, dynamic> cmd);

class EditorCommandHandler {
  static final Map<String, CommandExecutor> _handlers = {
    'layoutChanged': handleLayoutChanged,
    'renderRequired': handleRenderRequired,
    'cursorChanged': handleCursorChanged,
    'activeMarksChanged': handleMarksChanged,
    'selectionChanged': handleSelectionChanged,
    'externalElementChanged': handleExternalElements,
    'fontsRequired': handleFontsRequired,
    'writingSystemRequired': handleWritingSystem,
  };

  static void handleCommands(EditorController controller, List<dynamic>? commands) {
    if (commands == null) {
      return;
    }

    for (final cmd in commands) {
      final cmdMap = cmd as Map<String, dynamic>;
      final type = cmdMap['type'] as String?;
      final handler = _handlers[type];
      handler?.call(controller, cmdMap);
    }
  }
}
