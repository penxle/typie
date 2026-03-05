part of '../controller.dart';

class SelectionExpansionSemantic implements InteractionSemantic {
  SelectionHandleInfo? _anchor;
  Map<String, dynamic>? _initialRange;
  String? _wordSelectionBaselineRangeKey;
  bool _awaitingWordSelection = false;

  WordSelectionDragContext? get context {
    final anchor = _anchor;
    final initialRange = _initialRange;
    if (anchor == null || initialRange == null) {
      return null;
    }
    return (anchor: anchor, initialRange: initialRange);
  }

  void set({required SelectionHandleInfo anchor, required Map<String, dynamic> initialRange}) {
    _awaitingWordSelection = false;
    _wordSelectionBaselineRangeKey = null;
    _anchor = anchor;
    _initialRange = cloneSelectionRange(initialRange);
  }

  void prepareWordSelection({required String? baselineRangeKey}) {
    _awaitingWordSelection = true;
    _wordSelectionBaselineRangeKey = baselineRangeKey;
    _anchor = null;
    _initialRange = null;
  }

  bool adoptWordSelection(EditorSelection? selection) {
    if (!_awaitingWordSelection || selection == null || selection.collapsed) {
      return false;
    }

    final anchor = selection.fromBounds;
    if (anchor == null) {
      return false;
    }

    final rangeKey = rangeKeyFor(selection);
    if (rangeKey == null) {
      return false;
    }
    if (_wordSelectionBaselineRangeKey != null && _wordSelectionBaselineRangeKey == rangeKey) {
      return false;
    }

    set(anchor: anchor, initialRange: selection.range);
    return true;
  }

  String? rangeKeyFor(EditorSelection? selection) {
    if (selection == null) {
      return null;
    }
    return _selectionRangeKey(selection);
  }

  void clear() {
    _awaitingWordSelection = false;
    _wordSelectionBaselineRangeKey = null;
    _anchor = null;
    _initialRange = null;
  }

  @override
  void reset() {
    clear();
  }

  String? _selectionRangeKey(EditorSelection selection) {
    final anchor = selection.range['anchor'];
    final head = selection.range['head'];
    if (anchor is! Map || head is! Map) {
      return null;
    }

    final anchorMap = Map<String, dynamic>.from(anchor.cast<dynamic, dynamic>());
    final headMap = Map<String, dynamic>.from(head.cast<dynamic, dynamic>());

    String endpointKey(Map<String, dynamic> endpoint) {
      final nodeId = endpoint['nodeId']?.toString() ?? '';
      final offset = endpoint['offset']?.toString() ?? '';
      final affinity = endpoint['affinity']?.toString() ?? '';
      return '$nodeId:$offset:$affinity';
    }

    final collapsed = selection.collapsed ? '1' : '0';
    return '$collapsed|${selection.cmp}|${endpointKey(anchorMap)}|${endpointKey(headMap)}';
  }
}

extension SelectionExpansionSemanticActions on EditorInteractionController {
  bool _semanticSelectWordAt(Offset localPosition) {
    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    if (pageIdx < 0) {
      return false;
    }

    showContextMenu.value = false;
    scope.inputController
      ..invalidate()
      ..openInput();

    _semanticDispatchPrimaryClick(
      pageIdx: pageIdx,
      pointerX: _resolvePointerX(localPosition.dx),
      localY: localY,
      clickCount: 2,
      isShiftPressed: false,
    );
    scope.controller.scrollIntoView();
    return true;
  }

  void _semanticExtendSelectionTo({
    required SelectionHandleInfo anchor,
    required int headPageIdx,
    required double headX,
    required double headY,
    Map<String, dynamic>? initialRange,
  }) {
    scope.controller.dispatch(
      buildExtendSelectionEvent(
        anchor: anchor,
        headPageIdx: headPageIdx,
        headX: headX,
        headY: headY,
        initialRange: initialRange,
      ),
    );
  }
}
