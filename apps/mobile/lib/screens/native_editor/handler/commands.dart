import 'dart:async';

import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/state.dart';

void handleDocChanged(EditorController controller, Map<String, dynamic> cmd) {
  controller.onDocChanged?.call();
  controller.typewriterNeedsScroll = true;
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

  final hadLayout = controller.state.layout != null;

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

  if (!hadLayout && pageCount > 0) {
    controller.onEditorReady?.call();
  }
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
  final cursor = CursorInfo.fromMap(cmd);
  controller.updateState((state) => state.copyWith(cursor: cursor));
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

  final anchor = cmd['anchor'] as Map<String, dynamic>?;
  final head = cmd['head'] as Map<String, dynamic>?;
  if (anchor != null && head != null) {
    controller.onSelectionChanged?.call(anchor, head);
  }
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

void handleSearchResultsChanged(EditorController controller, Map<String, dynamic> cmd) {
  final totalCount = cmd['totalCount'] as int;
  final currentIndex = cmd['currentIndex'] as int;
  final overlays = cmd['overlays'] as List<dynamic>;

  SearchScrollTarget? scrollTarget;
  final searchOverlays = <SearchOverlayInfo>[];

  for (final overlay in overlays) {
    final map = overlay as Map<String, dynamic>;
    final pageIdx = map['pageIdx'] as int;
    final isCurrent = map['isCurrent'] as bool? ?? false;
    final bounds = map['bounds'] as List<dynamic>;

    final rects = bounds.map((b) {
      final m = b as Map<String, dynamic>;
      return SearchHighlightRect(
        x: (m['x'] as num).toDouble(),
        y: (m['y'] as num).toDouble(),
        width: (m['width'] as num).toDouble(),
        height: (m['height'] as num).toDouble(),
      );
    }).toList();

    searchOverlays.add(SearchOverlayInfo(pageIdx: pageIdx, isCurrent: isCurrent, bounds: rects));

    if (isCurrent && rects.isNotEmpty) {
      scrollTarget = SearchScrollTarget(
        pageIdx: pageIdx,
        x: rects[0].x,
        y: rects[0].y,
        width: rects[0].width,
        height: rects[0].height,
      );
    }
  }

  controller.updateState(
    (state) => state.copyWith(
      searchTotalCount: totalCount,
      searchCurrentIndex: currentIndex,
      searchScrollTarget: scrollTarget,
      searchOverlays: searchOverlays,
    ),
  );
}

void handleSpellcheckOverlaysChanged(EditorController controller, Map<String, dynamic> cmd) {
  final overlays = cmd['overlays'] as List<dynamic>;

  String? activeErrorId;
  SpellcheckOverlayBound? scrollTarget;
  int? scrollTargetPageIdx;
  final spellcheckOverlays = <SpellcheckOverlayInfo>[];

  for (final overlay in overlays) {
    final map = overlay as Map<String, dynamic>;
    final pageIdx = map['pageIdx'] as int;
    final id = map['id'] as String;
    final isActive = map['isActive'] as bool? ?? false;
    final bounds = map['bounds'] as List<dynamic>;

    final parsedBounds = bounds.map((b) {
      final m = b as Map<String, dynamic>;
      return SpellcheckOverlayBound(
        x: (m['x'] as num).toDouble(),
        y: (m['y'] as num).toDouble(),
        width: (m['width'] as num).toDouble(),
        height: (m['height'] as num).toDouble(),
        ascent: (m['ascent'] as num).toDouble(),
      );
    }).toList();

    spellcheckOverlays.add(SpellcheckOverlayInfo(pageIdx: pageIdx, id: id, isActive: isActive, bounds: parsedBounds));

    if (isActive) {
      activeErrorId = id;
      if (parsedBounds.isNotEmpty) {
        scrollTarget = parsedBounds[0];
        scrollTargetPageIdx = pageIdx;
      }
    }
  }

  controller.updateState(
    (state) => state.copyWith(
      spellcheckOverlays: spellcheckOverlays,
      activeSpellcheckErrorId: activeErrorId,
      spellcheckScrollTarget: scrollTarget,
      spellcheckScrollTargetPageIdx: scrollTargetPageIdx,
    ),
  );
}
