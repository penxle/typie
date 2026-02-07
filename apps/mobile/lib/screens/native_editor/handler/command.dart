import 'package:typie/screens/native_editor/handler/commands/cursor.dart';
import 'package:typie/screens/native_editor/handler/commands/document.dart';
import 'package:typie/screens/native_editor/handler/commands/font.dart';
import 'package:typie/screens/native_editor/handler/commands/layout.dart';
import 'package:typie/screens/native_editor/handler/commands/overlay.dart';
import 'package:typie/screens/native_editor/handler/commands/placeholder.dart';
import 'package:typie/screens/native_editor/handler/commands/selection.dart';
import 'package:typie/screens/native_editor/state/controller.dart';

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
    'fontRequired': handleFontRequired,
    'fallbackFontRequired': handleFallbackFontRequired,
    'exitedDocumentStart': handleExitedDocumentStart,
    'searchResultsChanged': handleSearchResultsChanged,
    'spellcheckOverlaysChanged': handleSpellcheckOverlaysChanged,
    'aiFeedbackOverlaysChanged': handleAiFeedbackOverlaysChanged,
    'placeholderChanged': handlePlaceholderChanged,
    'dropIndicatorChanged': handleDropIndicatorChanged,
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
