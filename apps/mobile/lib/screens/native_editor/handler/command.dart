import 'package:typie/screens/native_editor/handler/commands.dart';
import 'package:typie/screens/native_editor/state/state.dart';

typedef CommandExecutor = void Function(EditorController controller, Map<String, dynamic> cmd);

class CommandHandler {
  static final Map<String, CommandExecutor> _handlers = {
    'docChanged': handleDocChanged,
    'layoutChanged': handleLayoutChanged,
    'settingsChanged': handleSettingsChanged,
    'renderRequired': handleRenderRequired,
    'cursorChanged': handleCursorChanged,
    'activeMarksChanged': handleMarksChanged,
    'selectionChanged': handleSelectionChanged,
    'externalElementChanged': handleExternalElements,
    'fontsRequired': handleFontsRequired,
    'writingSystemRequired': handleWritingSystem,
    'exitedDocumentStart': handleExitedDocumentStart,
    'searchResultsChanged': handleSearchResultsChanged,
    'spellcheckOverlaysChanged': handleSpellcheckOverlaysChanged,
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
