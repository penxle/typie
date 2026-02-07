import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';

typedef _OverlayResult<T, B> = ({List<T> overlays, String? activeId, B? scrollTarget, int? scrollTargetPageIdx});

_OverlayResult<T, B> _parseOverlaysWithActive<T, B>({
  required List<dynamic> overlays,
  required T Function(int pageIdx, String id, bool isActive, List<B> bounds) createOverlay,
  required B Function(Map<String, dynamic> m) parseBound,
}) {
  String? activeId;
  B? scrollTarget;
  int? scrollTargetPageIdx;
  final parsed = <T>[];

  for (final overlay in overlays) {
    final map = overlay as Map<String, dynamic>;
    final pageIdx = map['pageIdx'] as int;
    final id = map['id'] as String;
    final isActive = map['isActive'] as bool? ?? false;
    final bounds = map['bounds'] as List<dynamic>;

    final parsedBounds = bounds.map((b) => parseBound(b as Map<String, dynamic>)).toList();
    parsed.add(createOverlay(pageIdx, id, isActive, parsedBounds));

    if (isActive) {
      activeId = id;
      if (parsedBounds.isNotEmpty) {
        scrollTarget = parsedBounds[0];
        scrollTargetPageIdx = pageIdx;
      }
    }
  }

  return (overlays: parsed, activeId: activeId, scrollTarget: scrollTarget, scrollTargetPageIdx: scrollTargetPageIdx);
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
      search: SearchState(
        totalCount: totalCount,
        currentIndex: currentIndex,
        scrollTarget: scrollTarget,
        overlays: searchOverlays,
      ),
    ),
  );
}

void handleSpellcheckOverlaysChanged(EditorController controller, Map<String, dynamic> cmd) {
  final result = _parseOverlaysWithActive<SpellcheckOverlayInfo, SpellcheckOverlayBound>(
    overlays: cmd['overlays'] as List<dynamic>,
    createOverlay: (pageIdx, id, isActive, bounds) =>
        SpellcheckOverlayInfo(pageIdx: pageIdx, id: id, isActive: isActive, bounds: bounds),
    parseBound: (m) => SpellcheckOverlayBound(
      x: (m['x'] as num).toDouble(),
      y: (m['y'] as num).toDouble(),
      width: (m['width'] as num).toDouble(),
      height: (m['height'] as num).toDouble(),
      ascent: (m['ascent'] as num).toDouble(),
    ),
  );

  controller.updateState(
    (state) => state.copyWith(
      spellcheck: SpellcheckState(
        overlays: result.overlays,
        activeErrorId: result.activeId,
        scrollTarget: result.scrollTarget,
        scrollTargetPageIdx: result.scrollTargetPageIdx,
      ),
    ),
  );
}

void handleAiFeedbackOverlaysChanged(EditorController controller, Map<String, dynamic> cmd) {
  final result = _parseOverlaysWithActive<AiFeedbackOverlayInfo, AiFeedbackOverlayBound>(
    overlays: cmd['overlays'] as List<dynamic>,
    createOverlay: (pageIdx, id, isActive, bounds) =>
        AiFeedbackOverlayInfo(pageIdx: pageIdx, id: id, isActive: isActive, bounds: bounds),
    parseBound: (m) => AiFeedbackOverlayBound(
      x: (m['x'] as num).toDouble(),
      y: (m['y'] as num).toDouble(),
      width: (m['width'] as num).toDouble(),
      height: (m['height'] as num).toDouble(),
    ),
  );

  controller.updateState(
    (state) => state.copyWith(
      aiFeedback: AiFeedbackState(
        overlays: result.overlays,
        activeItemId: result.activeId,
        scrollTarget: result.scrollTarget,
        scrollTargetPageIdx: result.scrollTargetPageIdx,
      ),
    ),
  );
}

void handleDropIndicatorChanged(EditorController controller, Map<String, dynamic> cmd) {
  final indicator = cmd['indicator'] as Map<String, dynamic>?;
  final info = indicator != null ? DropIndicatorInfo.fromMap(indicator) : null;
  controller.updateState((state) => state.copyWith(dropIndicator: info));
}
