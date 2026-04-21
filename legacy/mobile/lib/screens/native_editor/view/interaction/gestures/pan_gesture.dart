part of '../controller.dart';

class PanGesture implements InteractionGesture {
  Drag? _verticalDrag;
  Drag? _horizontalDrag;
  bool _horizontalPanEnabled = false;

  bool get hasScrollDrag => _verticalDrag != null || _horizontalDrag != null;

  void startDrag({
    required DragStartDetails details,
    required bool allowHorizontal,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics horizontalMetrics,
  }) {
    final horizontalPosition = horizontalMetrics.activePosition;
    final canStartHorizontal = allowHorizontal && horizontalMetrics.canScrollHorizontally && horizontalPosition != null;

    _horizontalPanEnabled = canStartHorizontal;
    final verticalPosition = resolveScrollPosition(verticalScrollController);
    if (verticalPosition != null) {
      _verticalDrag = verticalPosition.drag(details, () {
        _verticalDrag = null;
      });
    }

    if (canStartHorizontal) {
      _horizontalDrag = horizontalPosition.drag(details, () {
        _horizontalDrag = null;
      });
    }
  }

  void updateDrag(DragUpdateDetails details, {required HorizontalScrollMetrics horizontalMetrics}) {
    final horizontalPosition = horizontalMetrics.activePosition;
    final canFallbackHorizontal =
        _horizontalPanEnabled &&
        horizontalMetrics.canScrollHorizontally &&
        horizontalPosition != null &&
        details.delta.dx != 0;
    final horizontalBefore = canFallbackHorizontal ? horizontalPosition.pixels : null;

    _verticalDrag?.update(
      DragUpdateDetails(
        globalPosition: details.globalPosition,
        delta: Offset(0, details.delta.dy),
        primaryDelta: details.delta.dy,
        sourceTimeStamp: details.sourceTimeStamp,
      ),
    );

    _horizontalDrag?.update(
      DragUpdateDetails(
        globalPosition: details.globalPosition,
        delta: Offset(details.delta.dx, 0),
        primaryDelta: details.delta.dx,
        sourceTimeStamp: details.sourceTimeStamp,
      ),
    );

    if (canFallbackHorizontal) {
      final horizontalAfterDrag = horizontalPosition.pixels;
      final dragMoved = horizontalBefore != null && (horizontalAfterDrag - horizontalBefore).abs() > 0.01;
      if (!dragMoved) {
        final nextOffset = (horizontalAfterDrag - details.delta.dx).clamp(0.0, horizontalPosition.maxScrollExtent);
        if ((nextOffset - horizontalAfterDrag).abs() > 0) {
          horizontalPosition.jumpTo(nextOffset);
        }
      }
    }
  }

  void applyRawPanDelta({
    required Offset delta,
    required bool allowHorizontal,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics horizontalMetrics,
  }) {
    final verticalPosition = resolveScrollPosition(verticalScrollController);
    if (verticalPosition != null &&
        verticalPosition.hasContentDimensions &&
        verticalPosition.maxScrollExtent > 0 &&
        delta.dy != 0) {
      final currentOffset = verticalPosition.pixels;
      final nextOffset = (currentOffset - delta.dy).clamp(0.0, verticalPosition.maxScrollExtent);
      if ((nextOffset - currentOffset).abs() > 0) {
        verticalPosition.jumpTo(nextOffset);
      }
    }

    final horizontalPosition = horizontalMetrics.activePosition;
    if (allowHorizontal && horizontalMetrics.canScrollHorizontally && horizontalPosition != null && delta.dx != 0) {
      final currentOffset = horizontalPosition.pixels;
      final nextOffset = (currentOffset - delta.dx).clamp(0.0, horizontalPosition.maxScrollExtent);
      if ((nextOffset - currentOffset).abs() > 0) {
        horizontalPosition.jumpTo(nextOffset);
      }
    }
  }

  void endDrag(DragEndDetails details) {
    _verticalDrag?.end(
      DragEndDetails(
        velocity: Velocity(pixelsPerSecond: Offset(0, details.velocity.pixelsPerSecond.dy)),
        primaryVelocity: details.velocity.pixelsPerSecond.dy,
      ),
    );

    _horizontalDrag?.end(
      DragEndDetails(
        velocity: Velocity(pixelsPerSecond: Offset(details.velocity.pixelsPerSecond.dx, 0)),
        primaryVelocity: details.velocity.pixelsPerSecond.dx,
      ),
    );

    _verticalDrag = null;
    _horizontalDrag = null;
    _horizontalPanEnabled = false;
  }

  void cancelDrag() {
    _verticalDrag?.cancel();
    _horizontalDrag?.cancel();
    _verticalDrag = null;
    _horizontalDrag = null;
    _horizontalPanEnabled = false;
  }

  @override
  void reset() {
    cancelDrag();
  }
}

class PanResumeGesture implements InteractionGesture {
  int? pointer;
  Offset? lastLocalPosition;

  @override
  void reset() {
    pointer = null;
    lastLocalPosition = null;
  }
}

extension PanGestureMethods on EditorInteractionController {
  void onPanStart(DragStartDetails details) {
    final dndLocked = _consumeIfDndLocked(onLocked: _panGesture.cancelDrag);
    if (dndLocked) {
      return;
    }
    if (!_decide(command: InteractionCommand.panStart)) {
      return;
    }

    final next = _applyTransition(InteractionEvent.panStart);
    if (next.mode != InteractionMode.panning) {
      return;
    }

    _panGesture.startDrag(
      details: details,
      allowHorizontal: _allowHorizontalPan,
      verticalScrollController: scope.verticalScrollController,
      horizontalMetrics: _resolveHorizontalMetrics(),
    );
  }

  void onPanUpdate(DragUpdateDetails details) {
    final dndLocked = _consumeIfDndLocked(onLocked: _panGesture.cancelDrag);
    if (dndLocked) {
      return;
    }
    if (!_decide(command: InteractionCommand.panUpdate)) {
      return;
    }

    _panGesture.updateDrag(details, horizontalMetrics: _resolveHorizontalMetrics());
  }

  void onPanEnd(DragEndDetails details) {
    final dndLocked = _consumeIfDndLocked(onLocked: _panGesture.cancelDrag);
    if (dndLocked) {
      return;
    }
    if (!_decide(command: InteractionCommand.panEnd)) {
      return;
    }

    _applyTransition(InteractionEvent.panEnd);
    _panGesture.endDrag(details);
  }

  void onPanCancel() {
    final dndLocked = _consumeIfDndLocked(onLocked: _panGesture.cancelDrag);
    if (dndLocked) {
      return;
    }
    if (!_decide(command: InteractionCommand.panCancel)) {
      return;
    }

    _applyTransition(InteractionEvent.panCancel);
    _panGesture.cancelDrag();
  }
}
