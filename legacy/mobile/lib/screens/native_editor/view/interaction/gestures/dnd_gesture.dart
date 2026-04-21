part of '../controller.dart';

extension DndGestureMethods on EditorInteractionController {
  void _endNativeLocalDrag() {
    scope.dndController.handleDragEnd();
  }

  void onLocalDragCompleted(DropOperation operation) {
    final mode = scope.interactionState.snapshot().mode;
    if (mode != InteractionMode.dndLocal) {
      return;
    }

    if (operation == DropOperation.none ||
        operation == DropOperation.userCancelled ||
        operation == DropOperation.forbidden) {
      endDnd();
    }
  }

  void startLocalDnd(ResolvedDragLocation location) {
    if (!_decide(
      command: InteractionCommand.dndBeginLocal,
      transitionEvent: const InteractionEvent.dndStart(local: true),
      expectedMode: InteractionMode.dndLocal,
    )) {
      return;
    }

    scope.dndController.handleDragStart(
      location.pageIdx,
      location.pointerX,
      location.localY,
      Offset(location.localPosition.dx, location.localPosition.dy),
    );
  }

  void endDnd() {
    final wasLocal = scope.interactionState.snapshot().mode == InteractionMode.dndLocal;
    _applyTransition(InteractionEvent.dndSessionEnd);
    dropPosition.value = null;
    _autoScrollSemantic.stop();
    if (wasLocal) {
      _endNativeLocalDrag();
    }
  }

  DropOperation onDropOver(DropOverEvent event) {
    if (!_decide(command: InteractionCommand.dndHandleDropOver)) {
      return DropOperation.none;
    }
    _applyTransition(InteractionEvent.dndOver);

    final item = event.session.items.firstOrNull;
    if (!_decide(command: InteractionCommand.dndHandleDropOverItem(hasItem: item != null))) {
      return DropOperation.none;
    }
    final dropItem = item;
    if (dropItem == null) {
      return DropOperation.none;
    }

    final position = event.position.local;
    final (pageIdx, localY) = getPageAtPosition(position.dy);
    final pointerX = _resolvePointerX(position.dx);

    dropPosition.value = position;
    _handleAutoScroll(y: position.dy, x: position.dx);
    scope.dndController.handleDragOver(pageIdx, pointerX, localY);

    final localData = dropItem.localData;
    if (localData is Map && localData['isInternal'] == true) {
      return DropOperation.move;
    }
    return DropOperation.copy;
  }

  void onDropEnter(dynamic event) {
    if (!_decide(command: InteractionCommand.dndHandleDropEnter)) {
      return;
    }

    if (!_decide(command: InteractionCommand.dndBeginExternal, transitionEvent: InteractionEvent.dndEnter)) {
      return;
    }

    scope.dndController.handleDragEnter();
  }

  void onDropLeave(dynamic event) {
    _applyTransition(InteractionEvent.dndLeave);
    dropPosition.value = null;
    _autoScrollSemantic.stop();
    scope.dndController.handleDragLeave();
  }

  void onDropEnded(dynamic event) {
    if (_decide(command: InteractionCommand.dndShouldEndOnDropEnded)) {
      endDnd();
      return;
    }
  }

  Future<void> onPerformDrop(PerformDropEvent event) async {
    if (!_decide(command: InteractionCommand.dndPerformDrop)) {
      return;
    }

    dropPosition.value = null;
    _autoScrollSemantic.stop();

    final position = event.position.local;
    final (pageIdx, localY) = getPageAtPosition(position.dy);
    if (!_decide(command: InteractionCommand.dndPerformDropOnPage(pageIdx: pageIdx))) {
      endDnd();
      return;
    }

    final wasLocal = scope.interactionState.snapshot().mode == InteractionMode.dndLocal;
    _applyTransition(InteractionEvent.dndDrop);

    final pointerX = _resolvePointerX(position.dx);
    unawaited(HapticFeedback.lightImpact());
    final result = await scope.dndController.handleDrop(
      pageIdx: pageIdx,
      x: pointerX,
      y: localY,
      session: event.session,
    );
    var endedDrag = false;
    if (result == DndDropResult.needsDragEnd) {
      _endNativeLocalDrag();
      endedDrag = true;
    }
    if (wasLocal && !endedDrag) {
      _endNativeLocalDrag();
    }
  }
}
