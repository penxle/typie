part of '../controller.dart';

class TapGesture implements InteractionGesture {
  DateTime? lastTapTime;
  Offset? lastTapPosition;
  Timer? tapTimer;
  bool tapDispatched = false;

  bool isConsecutiveTap({
    required Offset localPosition,
    required DateTime now,
    int maxTapIntervalMs = 300,
    double maxTapDistance = 20,
  }) {
    final prevTime = lastTapTime;
    final prevPosition = lastTapPosition;
    if (prevTime == null || prevPosition == null) {
      return false;
    }

    final timeDiff = now.difference(prevTime).inMilliseconds;
    final distance = (localPosition - prevPosition).distance;
    return timeDiff < maxTapIntervalMs && distance < maxTapDistance;
  }

  void recordTap({required DateTime now, required Offset localPosition}) {
    lastTapTime = now;
    lastTapPosition = localPosition;
  }

  void clearTapHistory() {
    lastTapTime = null;
    lastTapPosition = null;
  }

  void cancelTapTimer() {
    tapTimer?.cancel();
    tapTimer = null;
  }

  void scheduleTapTimer(Duration duration, VoidCallback onTimeout) {
    cancelTapTimer();
    tapTimer = Timer(duration, onTimeout);
  }

  @override
  void reset() {
    cancelTapTimer();
    clearTapHistory();
    tapDispatched = false;
  }
}

extension TapGestureMethods on EditorInteractionController {
  bool _isConsecutiveTap({required Offset localPosition, required DateTime now}) {
    return _tapGesture.isConsecutiveTap(localPosition: localPosition, now: now);
  }

  void _dispatchTap(Offset localPosition) {
    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    if (!_decide(command: InteractionCommand.tapDispatch(pageIdx: pageIdx))) {
      return;
    }

    showContextMenu.value = false;

    scope.inputController
      ..invalidate()
      ..openInput();

    final now = DateTime.now();
    final clickCount = _isConsecutiveTap(localPosition: localPosition, now: now) ? 2 : 1;

    _tapGesture.recordTap(now: now, localPosition: localPosition);

    final pointerX = _resolvePointerX(localPosition.dx);

    final hitOverlay = scope.controller.interactiveOverlays.firstWhereOrNull(
      (o) => o.hitTest(pageIdx, pointerX, localY),
    );
    if (hitOverlay != null) {
      _semanticDispatchOverlayAction(hitOverlay);
      return;
    }

    if (clickCount == 1) {
      final isSelectionHit = scope.editor.isSelectionHit(pageIdx, pointerX, localY);
      if (isSelectionHit) {
        if (!wasContextMenuOpen.value) {
          showContextMenu.value = true;
        }
        return;
      }
    }

    final keysPressed = HardwareKeyboard.instance.logicalKeysPressed;
    final isShiftHeader =
        keysPressed.contains(LogicalKeyboardKey.shiftLeft) || keysPressed.contains(LogicalKeyboardKey.shiftRight);

    final prevCursor = scope.controller.state.cursor;

    _semanticDispatchPrimaryClick(
      pageIdx: pageIdx,
      pointerX: pointerX,
      localY: localY,
      clickCount: clickCount,
      isShiftPressed: isShiftHeader,
    );

    if (clickCount != 1) {
      scope.controller.scrollIntoView();
      return;
    }

    unawaited(
      scope.ticker.settled().then((_) {
        if (!context.mounted) {
          return;
        }

        final newState = scope.controller.state;
        final isCollapsed = newState.selection?.collapsed ?? true;

        final isSameCursor =
            isCollapsed && newState.cursor != null && prevCursor != null && newState.cursor!.isSamePosition(prevCursor);

        if (isSameCursor) {
          if (!wasContextMenuOpen.value) {
            showContextMenu.value = true;
          }
          return;
        }

        scope.controller.scrollIntoView();
      }),
    );
  }

  void onTapDown(TapDownDetails details) {
    if (!_decide(command: InteractionCommand.tapDown)) {
      return;
    }

    wasContextMenuOpen.value = showContextMenu.value;
    if (showContextMenu.value) {
      showContextMenu.value = false;
    }

    _tapGesture.cancelTapTimer();

    if (_isConsecutiveTap(localPosition: details.localPosition, now: DateTime.now())) {
      _tapGesture.tapDispatched = true;
      if (_dispatchDoubleTapSelection(details.localPosition)) {
        prepareDoubleTapDrag(details.localPosition);
      }
      return;
    }

    _tapGesture
      ..tapDispatched = false
      ..scheduleTapTimer(const Duration(milliseconds: 150), () {
        final pointerX = _resolvePointerX(details.localPosition.dx);
        final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);

        final canDrag = scope.editor.isSelectionHit(pageIdx, pointerX, localY);
        if (canDrag) {
          _tapGesture.tapDispatched = true;
          return;
        }

        final hasRangeSelection = !(scope.controller.state.selection?.collapsed ?? true);
        if (hasRangeSelection) {
          return;
        }

        _tapGesture.tapDispatched = true;
        _dispatchTap(details.localPosition);
      });
  }

  void onTapUp(TapUpDetails details) {
    if (!_decide(command: InteractionCommand.tapUp)) {
      return;
    }

    _doubleTapDragGesture.clearPending();
    _tapGesture.cancelTapTimer();
    if (!_tapGesture.tapDispatched) {
      _dispatchTap(details.localPosition);
    }
  }

  void onTapCancel() {
    if (!_decide(command: InteractionCommand.tapCancel)) {
      return;
    }

    _doubleTapDragGesture.clearPending();
    _tapGesture.cancelTapTimer();
  }
}
