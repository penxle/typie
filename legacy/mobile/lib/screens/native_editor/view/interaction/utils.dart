part of 'controller.dart';

Map<String, dynamic> cloneSelectionRange(Map<String, dynamic> range) {
  final cloned = Map<String, dynamic>.from(range);
  final anchor = cloned['anchor'];
  if (anchor is Map) {
    cloned['anchor'] = Map<String, dynamic>.from(anchor.cast<dynamic, dynamic>());
  }
  final head = cloned['head'];
  if (head is Map) {
    cloned['head'] = Map<String, dynamic>.from(head.cast<dynamic, dynamic>());
  }
  return cloned;
}

Map<String, dynamic> buildExtendSelectionEvent({
  required SelectionHandleInfo anchor,
  required int headPageIdx,
  required double headX,
  required double headY,
  Map<String, dynamic>? initialRange,
}) {
  return {
    'type': 'extendSelectionTo',
    'anchorPageIdx': anchor.pageIdx,
    'anchorX': anchor.x,
    'anchorY': anchor.y + anchor.height / 2,
    'headPageIdx': headPageIdx,
    'headX': headX,
    'headY': headY,
    'doubleTapInitialRange': ?initialRange,
  };
}

Map<String, dynamic> buildPrimaryPointerDownEvent({
  required int pageIdx,
  required double pointerX,
  required double localY,
  required int clickCount,
  required bool isShiftPressed,
}) {
  return {
    'type': 'pointerDown',
    'pageIdx': pageIdx,
    'x': pointerX,
    'y': localY,
    'clickCount': clickCount,
    'button': 'primary',
    'modifier': {'shift': isShiftPressed, 'ctrl': false, 'alt': false, 'meta': false},
  };
}

Map<String, dynamic> buildPrimaryPointerUpEvent({
  required int pageIdx,
  required double pointerX,
  required double localY,
  required bool isShiftPressed,
}) {
  return {
    'type': 'pointerUp',
    'pageIdx': pageIdx,
    'x': pointerX,
    'y': localY,
    'button': 'primary',
    'modifier': {'shift': isShiftPressed, 'ctrl': false, 'alt': false, 'meta': false},
  };
}
