part of '../controller.dart';

class ConditionalLongPressGestureRecognizer extends LongPressGestureRecognizer {
  ConditionalLongPressGestureRecognizer({required this.condition, super.duration, super.postAcceptSlopTolerance});

  final bool Function(Offset globalPosition) condition;

  @override
  void didExceedDeadline() {
    if (initialPosition == null) {
      super.didExceedDeadline();
      return;
    }

    final globalPosition = initialPosition!.global;
    if (condition(globalPosition)) {
      resolve(GestureDisposition.rejected);
      stopTrackingPointer(primaryPointer!);
    } else {
      super.didExceedDeadline();
    }
  }
}

class LongPressGesture implements InteractionGesture {
  bool _active = false;
  bool? _androidUseCursorModeAtPointerDown;

  bool get active => _active;
  bool? get androidUseCursorModeAtPointerDown => _androidUseCursorModeAtPointerDown;

  void primeAndroidSemanticAtPointerDown({required bool useCursorMode}) {
    _androidUseCursorModeAtPointerDown = useCursorMode;
  }

  bool begin() {
    if (_active) {
      return false;
    }
    _active = true;
    return true;
  }

  void end() {
    _active = false;
    _androidUseCursorModeAtPointerDown = null;
  }

  @override
  void reset() {
    _active = false;
    _androidUseCursorModeAtPointerDown = null;
  }
}

enum _LongPressSemanticIntent { cursorMove, wordSelection }

extension LongPressGestureMethods on EditorInteractionController {
  _LongPressSemanticIntent _resolveLongPressSemanticIntent(Offset localPosition) {
    if (!_isAndroid) {
      return _LongPressSemanticIntent.cursorMove;
    }

    final useCursorMode =
        _longPressGesture.androidUseCursorModeAtPointerDown ?? _shouldUseAndroidCursorLongPress(localPosition);
    return useCursorMode ? _LongPressSemanticIntent.cursorMove : _LongPressSemanticIntent.wordSelection;
  }

  bool _beginLongPressSemantic(_LongPressSemanticIntent semanticIntent) {
    if (semanticIntent != _LongPressSemanticIntent.wordSelection) {
      return true;
    }

    _selectionExpansionSemantic.prepareWordSelection(
      baselineRangeKey: _selectionExpansionSemantic.rangeKeyFor(scope.controller.state.selection),
    );
    return true;
  }

  bool startLongPress(Offset globalPosition) {
    final viewportPosition = viewportPositionFromGlobal(globalPosition);
    final admission = _resolveLongPressAdmission(viewportPosition: viewportPosition, runtime: _runtimeRead());
    if (admission == null) {
      return false;
    }

    scope.inputController.commitComposing();

    final localPosition = admission.viewportPosition;
    final semanticIntent = admission.semanticIntent;

    if (!_longPressGesture.begin()) {
      return false;
    }

    if (!_beginLongPressSemantic(semanticIntent)) {
      _longPressGesture.end();
      return false;
    }

    longPressPosition.value = viewportPosition;

    final event = semanticIntent == _LongPressSemanticIntent.wordSelection
        ? InteractionEvent.longPressWordStart
        : InteractionEvent.longPressStart;
    final expectedMode = semanticIntent == _LongPressSemanticIntent.wordSelection
        ? InteractionMode.longPressWordSelecting
        : InteractionMode.longPressSelecting;

    final next = _applyTransition(event);
    if (next.mode != expectedMode) {
      _clearLongPressState();
      _selectionExpansionSemantic.clear();
      return false;
    }

    if (semanticIntent == _LongPressSemanticIntent.wordSelection) {
      if (_semanticSelectWordAt(localPosition)) {
        _tapGesture.clearTapHistory();
      }
    }
    return true;
  }

  bool updateLongPress(Offset viewportPosition) {
    if (!_decide(command: InteractionCommand.longPressUpdate)) {
      return false;
    }

    final interactionMode = scope.interactionState.snapshot().mode;
    final isWordSelectionMode = interactionMode == InteractionMode.longPressWordSelecting;
    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
    longPressPosition.value = viewportPosition;

    if (pageIdx >= 0) {
      final pointerX = _resolvePointerX(viewportPosition.dx);
      if (isWordSelectionMode) {
        final selectionContext = _resolveLongPressWordSelectionContext();
        if (selectionContext != null) {
          _semanticExtendSelectionTo(
            anchor: selectionContext.anchor,
            headPageIdx: pageIdx,
            headX: pointerX,
            headY: localY,
            initialRange: selectionContext.initialRange,
          );
        }
      } else {
        _semanticMoveCursorAt(pageIdx: pageIdx, pointerX: pointerX, localY: localY);
      }
      scope.controller.scrollIntoView();
    }

    _handleAutoScroll(y: viewportPosition.dy, x: viewportPosition.dx);
    return true;
  }

  bool _shouldUseAndroidCursorLongPress(Offset localPosition) {
    final cursor = scope.controller.state.cursor;
    if (cursor == null || !cursor.visible) {
      return false;
    }

    final selection = scope.controller.state.selection;
    final isCollapsed = selection?.collapsed ?? true;
    if (!isCollapsed) {
      return false;
    }

    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    if (pageIdx < 0) {
      return false;
    }

    final pointerX = _resolvePointerX(localPosition.dx);
    return scope.editor.isCursorHit(pageIdx, pointerX, localY);
  }

  void primeLongPressModeAtPointerDown(Offset localPosition) {
    if (!_isAndroid) {
      return;
    }
    _longPressGesture.primeAndroidSemanticAtPointerDown(useCursorMode: _shouldUseAndroidCursorLongPress(localPosition));
  }

  bool endLongPress() {
    if (!_decide(command: InteractionCommand.longPressEnd)) {
      return false;
    }

    final endedWord = scope.interactionState.snapshot().mode == InteractionMode.longPressWordSelecting;
    _applyTransition(endedWord ? InteractionEvent.longPressWordEnd : InteractionEvent.longPressEnd);
    return true;
  }
}
