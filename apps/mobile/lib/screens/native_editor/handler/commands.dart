import 'dart:async';

import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/fonts.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

void handleLayoutChanged(EditorController controller, Map<String, dynamic> cmd) {
  final pageCount = cmd['pageCount'] as int;
  final layoutMode = cmd['layoutMode'] as Map<String, dynamic>;
  final pageHeights = cmd['pageHeights'] as List<dynamic>;

  controller.updateState(
    (state) => state.copyWith(
      layout: LayoutInfo(
        pageCount: pageCount,
        isPaginated: layoutMode['type'] == 'paginated',
        pageHeights: pageHeights.cast<num>().map((e) => e.toDouble()).toList(),
      ),
    ),
  );
}

void handleRenderRequired(EditorController controller, Map<String, dynamic> cmd) {
  controller.updateState((state) => state.copyWith(renderVersion: Object()));
}

void handleCursorChanged(EditorController controller, Map<String, dynamic> cmd) {
  controller.updateState((state) => state.copyWith(cursor: CursorInfo.fromMap(cmd)));
}

void handleMarksChanged(EditorController controller, Map<String, dynamic> cmd) {
  final uniform = cmd['uniformMarks'] as List<dynamic>;
  final mixed = cmd['mixedMarks'] as List<dynamic>;

  controller.updateState(
    (state) => state.copyWith(uniformMarks: uniform.cast<Map<String, dynamic>>(), mixedMarks: mixed.cast<String>()),
  );
}

void handleSelectionChanged(EditorController controller, Map<String, dynamic> cmd) {
  final stats = cmd['stats'] as Map<String, dynamic>;
  controller.updateState((state) => state.copyWith(selectionStats: stats));
}

void handleExternalElements(EditorController controller, Map<String, dynamic> cmd) {
  final elements = cmd['elements'] as List<dynamic>;
  controller.updateState(
    (state) => state.copyWith(
      externalElements: elements.map((e) => ExternalElement.fromJson(e as Map<String, dynamic>)).toList(),
    ),
  );
}

void handleFontsRequired(EditorController controller, Map<String, dynamic> cmd) {
  final manager = controller.fontManager;
  if (manager == null || manager.pendingFontLoad) {
    return;
  }

  final fonts = cmd['fonts'] as List<dynamic>;
  manager.pendingFontLoad = true;
  final fontList = fonts.cast<List<dynamic>>().map((f) => (f[0] as String, (f[1] as num).toInt())).toList();

  unawaited(
    manager.ensureRequiredFonts(fontList).then((loaded) {
      manager.pendingFontLoad = false;
      if (loaded) {
        controller.dispatch({'type': 'fontsLoaded'});
      }
    }),
  );
}

void handleWritingSystem(EditorController controller, Map<String, dynamic> cmd) {
  final manager = controller.fontManager;
  if (manager == null) {
    return;
  }

  final systems = cmd['systems'] as List<dynamic>;
  final systemList = systems.cast<String>().map((s) => WritingSystem.values.firstWhere((ws) => ws.name == s)).toList();

  unawaited(
    manager.ensureRequiredScripts(systemList).then((loaded) {
      if (loaded) {
        controller.dispatch({'type': 'fontsLoaded'});
      }
    }),
  );
}
