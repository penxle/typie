part of '../controller.dart';

class CursorMoveSemantic implements InteractionSemantic {
  @override
  void reset() {}
}

extension CursorMoveSemanticActions on EditorInteractionController {
  void _semanticDispatchOverlayAction(InteractiveOverlayRaw overlay) {
    if (overlay.kind == 0) {
      scope.controller.dispatch({'type': 'toggleFold', 'nodeId': overlay.nodeId});
      return;
    }
    if (overlay.kind == 1) {
      scope.controller.dispatch({'type': 'cycleCalloutVariantAt', 'nodeId': overlay.nodeId});
    }
  }

  void _semanticDispatchPrimaryClick({
    required int pageIdx,
    required double pointerX,
    required double localY,
    required int clickCount,
    required bool isShiftPressed,
  }) {
    scope.controller.dispatch(
      buildPrimaryPointerDownEvent(
        pageIdx: pageIdx,
        pointerX: pointerX,
        localY: localY,
        clickCount: clickCount,
        isShiftPressed: isShiftPressed,
      ),
    );
    scope.controller.dispatch(
      buildPrimaryPointerUpEvent(pageIdx: pageIdx, pointerX: pointerX, localY: localY, isShiftPressed: isShiftPressed),
    );
  }

  void _semanticMoveCursorAt({required int pageIdx, required double pointerX, required double localY}) {
    _semanticDispatchPrimaryClick(
      pageIdx: pageIdx,
      pointerX: pointerX,
      localY: localY,
      clickCount: 1,
      isShiftPressed: false,
    );
  }
}
