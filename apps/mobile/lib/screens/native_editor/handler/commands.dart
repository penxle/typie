import 'dart:async';

import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/fonts.dart';
import 'package:typie/screens/native_editor/selection_handle.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

void handleDocChanged(EditorController controller, Map<String, dynamic> cmd) {
  controller.onDocChanged?.call();
}

void handleLayoutChanged(EditorController controller, Map<String, dynamic> cmd) {
  final pageCount = cmd['pageCount'] as int;
  final layoutModeMap = cmd['layoutMode'] as Map<String, dynamic>;
  final pageWidth = (cmd['pageWidth'] as num).toDouble();
  final pageHeights = cmd['pageHeights'] as List<dynamic>;

  final isPaginated = layoutModeMap['type'] == 'paginated';
  final LayoutModeInfo layoutMode;

  if (isPaginated) {
    layoutMode = LayoutModeInfo.paginated(
      pageWidth: (layoutModeMap['pageWidth'] as num).toDouble(),
      pageHeight: (layoutModeMap['pageHeight'] as num).toDouble(),
      pageMarginTop: (layoutModeMap['pageMarginTop'] as num).toDouble(),
      pageMarginBottom: (layoutModeMap['pageMarginBottom'] as num).toDouble(),
      pageMarginLeft: (layoutModeMap['pageMarginLeft'] as num).toDouble(),
      pageMarginRight: (layoutModeMap['pageMarginRight'] as num).toDouble(),
    );
  } else {
    layoutMode = LayoutModeInfo.continuous(maxWidth: (layoutModeMap['maxWidth'] as num).toDouble());
  }

  controller.updateState(
    (state) => state.copyWith(
      layout: LayoutInfo(
        pageCount: pageCount,
        isPaginated: isPaginated,
        pageWidth: pageWidth,
        pageHeights: pageHeights.cast<num>().map((e) => e.toDouble()).toList(),
        layoutMode: layoutMode,
      ),
      renderVersion: Object(),
    ),
  );
}

void handleSettingsChanged(EditorController controller, Map<String, dynamic> cmd) {
  final paragraphIndent = (cmd['paragraphIndent'] as num).toDouble();
  final blockGap = (cmd['blockGap'] as num).toDouble();

  controller.updateState(
    (state) => state.copyWith(
      settings: state.settings.copyWith(paragraphIndent: paragraphIndent, blockGap: blockGap),
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
  final fromHandleMap = cmd['fromHandle'] as Map<String, dynamic>?;
  final toHandleMap = cmd['toHandle'] as Map<String, dynamic>?;

  final fromHandle = fromHandleMap != null ? SelectionHandleInfo.fromMap(fromHandleMap) : null;
  final toHandle = toHandleMap != null ? SelectionHandleInfo.fromMap(toHandleMap) : null;

  controller.updateState((state) => state.copyWith(selectionStats: stats, fromHandle: fromHandle, toHandle: toHandle));
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
    manager
        .ensureRequiredFonts(
          fontList,
          callbacks: (onStart: controller.incrementFontLoading, onEnd: controller.decrementFontLoading),
        )
        .then((loaded) {
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
    manager
        .ensureRequiredWritingSystems(
          systemList,
          callbacks: (onStart: controller.incrementFontLoading, onEnd: controller.decrementFontLoading),
        )
        .then((loaded) {
          if (loaded) {
            controller.dispatch({'type': 'fontsLoaded'});
          }
        }),
  );
}

void handleExitedDocumentStart(EditorController controller, Map<String, dynamic> cmd) {
  controller.onExitedDocumentStart?.call();
}
