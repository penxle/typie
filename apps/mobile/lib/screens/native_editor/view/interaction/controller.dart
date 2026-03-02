import 'dart:async';
import 'dart:math' as math;

import 'package:collection/collection.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/screens/native_editor/controller/dnd_controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/interaction/core.dart';
import 'package:typie/screens/native_editor/view/interaction/session.dart';
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

part 'sessions/auto_scroll_session.dart';
part 'sessions/dnd_session.dart';
part 'sessions/double_tap_drag_session.dart';
part 'sessions/handle_drag_session.dart';
part 'sessions/long_press_session.dart';
part 'sessions/pan_session.dart';
part 'sessions/pinch_session.dart';
part 'sessions/tap_session.dart';

typedef PagePositionResolver = (int pageIdx, double localY) Function(double y);
typedef ResolvedDragLocation = ({Offset localPosition, int pageIdx, double localY, double pointerX});

class EditorInteractionRuntime {
  const EditorInteractionRuntime({
    required this.controller,
    required this.interactionRegionKey,
    required this.dropPosition,
  });

  final EditorInteractionController controller;
  final GlobalKey interactionRegionKey;
  final ValueNotifier<Offset?> dropPosition;
}

EditorInteractionRuntime useEditorInteractionRuntime({
  required BuildContext context,
  required ContentScope scope,
  required ValueNotifier<bool> showContextMenu,
  required ObjectRef<bool> wasContextMenuOpen,
  required ValueNotifier<Size> viewportSize,
}) {
  final dropPosition = useValueNotifier<Offset?>(null);
  final interactionRegionKey = useMemoized(GlobalKey.new);

  final tapSession = useMemoized(TapSession.new);
  final doubleTapDragSession = useMemoized(DoubleTapDragSession.new);
  final longPressSession = useMemoized(LongPressSession.new);
  final panSession = useMemoized(PanSession.new);
  final panResumeSession = useMemoized(PanResumeSession.new);
  final pinchViewportSession = useMemoized(PinchViewportSession.new);
  final pinchSession = useMemoized(() => PinchSession(viewportSession: pinchViewportSession), [pinchViewportSession]);
  final handleDragSession = useMemoized(HandleDragSession.new);
  final autoScrollSession = useMemoized(AutoScrollSession.new);
  final dndSession = useMemoized(DndSession.new);

  final sessions = useMemoized(
    () => EditorInteractionSessions(
      tap: tapSession,
      doubleTapDrag: doubleTapDragSession,
      longPress: longPressSession,
      pan: panSession,
      panResume: panResumeSession,
      pinch: pinchSession,
      pinchViewport: pinchViewportSession,
      handleDrag: handleDragSession,
      autoScroll: autoScrollSession,
      dnd: dndSession,
    ),
    [
      tapSession,
      doubleTapDragSession,
      longPressSession,
      panSession,
      panResumeSession,
      pinchSession,
      pinchViewportSession,
      handleDragSession,
      autoScrollSession,
      dndSession,
    ],
  );

  final uiState = useMemoized(
    () => EditorInteractionUiState(
      showContextMenu: showContextMenu,
      wasContextMenuOpen: wasContextMenuOpen,
      longPressPosition: scope.longPressPosition,
      handleDragPosition: scope.handleDragPosition,
      dropPosition: dropPosition,
    ),
    [showContextMenu, wasContextMenuOpen, scope.longPressPosition, scope.handleDragPosition, dropPosition],
  );

  final interactionController = useMemoized(
    () => EditorInteractionController(
      context: context,
      interactionRegionKey: interactionRegionKey,
      scope: scope,
      sessions: sessions,
      getPageAtPosition: (y) => _resolvePageAtPosition(scope, y),
      ui: uiState,
      readViewWidth: () => viewportSize.value.width,
      readViewHeight: () => viewportSize.value.height,
      readGeometry: () => scope.geometry,
    ),
    [
      interactionRegionKey,
      scope.controller,
      scope.inputController,
      scope.interactionState,
      scope.verticalScrollController,
      scope.horizontalScrollController,
      scope.longPressPosition,
      scope.handleDragPosition,
      scope.titleAreaHeight,
      scope.displayZoom,
      scope.renderZoom,
      viewportSize,
      sessions,
      uiState,
    ],
  );

  useEffect(() => sessions.reset, [sessions]);

  return EditorInteractionRuntime(
    controller: interactionController,
    interactionRegionKey: interactionRegionKey,
    dropPosition: dropPosition,
  );
}

(int pageIdx, double localY) _resolvePageAtPosition(ContentScope scope, double y) {
  final geometry = scope.geometry;
  final offsets = geometry.computeCumulativePageOffsets();
  final scrollOffset = resolveScrollOffset(scope.verticalScrollController);
  final absoluteY = y + scrollOffset;
  final extensionAreaTop = (geometry.titleAreaHeight - geometry.toDisplayY(ContentGeometry.pagePadding)).clamp(
    0.0,
    double.infinity,
  );

  if (absoluteY < extensionAreaTop) {
    return (-1, absoluteY);
  }

  final adjustedY = absoluteY - geometry.titleAreaHeight;

  var low = 0;
  var high = offsets.length - 1;
  while (low < high) {
    final mid = (low + high) ~/ 2;
    if (offsets[mid] <= adjustedY) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }

  final pageIdx = (low - 1).clamp(0, geometry.pages.length - 1);
  final localY = geometry.toLogicalY(adjustedY - offsets[pageIdx]);
  return (pageIdx, localY);
}

class EditorInteractionControllerScope extends InheritedWidget {
  const EditorInteractionControllerScope({required this.controller, required super.child, super.key});

  final EditorInteractionController controller;

  static EditorInteractionController? maybeOf(BuildContext context) {
    return context.dependOnInheritedWidgetOfExactType<EditorInteractionControllerScope>()?.controller;
  }

  static EditorInteractionController of(BuildContext context) {
    final scope = maybeOf(context);
    assert(scope != null, 'EditorInteractionControllerScope not found in widget tree');
    return scope!;
  }

  @override
  bool updateShouldNotify(covariant EditorInteractionControllerScope oldWidget) {
    return !identical(oldWidget.controller, controller);
  }
}

class EditorInteractionSessions {
  const EditorInteractionSessions({
    required this.tap,
    required this.doubleTapDrag,
    required this.longPress,
    required this.pan,
    required this.panResume,
    required this.pinch,
    required this.pinchViewport,
    required this.handleDrag,
    required this.autoScroll,
    required this.dnd,
  });

  final TapSession tap;
  final DoubleTapDragSession doubleTapDrag;
  final LongPressSession longPress;
  final PanSession pan;
  final PanResumeSession panResume;
  final PinchSession pinch;
  final PinchViewportSession pinchViewport;
  final HandleDragSession handleDrag;
  final AutoScrollSession autoScroll;
  final DndSession dnd;

  void reset() {
    tap.reset();
    doubleTapDrag.reset();
    longPress.reset();
    pan.reset();
    panResume.reset();
    pinch.reset();
    handleDrag.reset();
    autoScroll.reset();
    dnd.reset();
  }
}

class EditorInteractionUiState {
  const EditorInteractionUiState({
    required this.showContextMenu,
    required this.wasContextMenuOpen,
    required this.longPressPosition,
    required this.handleDragPosition,
    required this.dropPosition,
  });

  final ValueNotifier<bool> showContextMenu;
  final ObjectRef<bool> wasContextMenuOpen;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<Offset?> dropPosition;
}

class EditorInteractionController {
  EditorInteractionController({
    required this.context,
    required this.interactionRegionKey,
    required this.scope,
    required EditorInteractionSessions sessions,
    required this.getPageAtPosition,
    required EditorInteractionUiState ui,
    required this.readViewWidth,
    required this.readViewHeight,
    required this.readGeometry,
  }) : _tapSession = sessions.tap,
       _doubleTapDragSession = sessions.doubleTapDrag,
       _longPressSession = sessions.longPress,
       _panSession = sessions.pan,
       _panResumeSession = sessions.panResume,
       pinchSession = sessions.pinch,
       pinchViewportSession = sessions.pinchViewport,
       _handleDragSession = sessions.handleDrag,
       _autoScrollSession = sessions.autoScroll,
       _dndSession = sessions.dnd,
       showContextMenu = ui.showContextMenu,
       wasContextMenuOpen = ui.wasContextMenuOpen,
       longPressPosition = ui.longPressPosition,
       handleDragPosition = ui.handleDragPosition,
       dropPosition = ui.dropPosition;

  final BuildContext context;
  final GlobalKey interactionRegionKey;
  final ContentScope scope;
  final TapSession _tapSession;
  final DoubleTapDragSession _doubleTapDragSession;
  final LongPressSession _longPressSession;
  final PanSession _panSession;
  final PanResumeSession _panResumeSession;
  final PinchSession pinchSession;
  final PinchViewportSession pinchViewportSession;
  final HandleDragSession _handleDragSession;
  final AutoScrollSession _autoScrollSession;
  final DndSession _dndSession;
  final PagePositionResolver getPageAtPosition;
  final ValueNotifier<bool> showContextMenu;
  final ObjectRef<bool> wasContextMenuOpen;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<Offset?> dropPosition;
  final double Function() readViewWidth;
  final double Function() readViewHeight;
  final ContentGeometry Function() readGeometry;

  bool get interactionActive => _doubleTapDragSession.active;
  bool get hasSelectionHandleDrag => _handleDragSession.hasSelectionHandleDrag;
  bool get isTableCellHandleDragging => _handleDragSession.isCellHandleDragging;

  Offset? selectionHandleViewportPosition(SelectionHandleInfo? handle, ContentGeometry geometry) {
    return _handleDragSession.getHandlePosition(
      handle,
      geometry,
      verticalScrollController: scope.verticalScrollController,
      resolveHorizontalMetrics: _resolveHorizontalMetrics,
    );
  }

  void stopSelectionHandlesAndAutoScroll() {
    _handleDragSession.clearSelectionHandleState();
    _autoScrollSession.stop();
  }

  void stopInteractionAutoScroll() {
    _autoScrollSession.stop();
  }

  void cancelInteractionScrollDrag() {
    _panSession.cancelDrag();
  }

  bool startAuxiliaryGesture(AuxiliaryGestureKind kind) {
    return _decide(
      command: InteractionCommand.auxiliaryBegin,
      transitionEvent: InteractionEvent.auxiliaryGestureStart(kind: kind),
      expectedMode: InteractionMode.auxiliaryGesture,
    );
  }

  bool updateAuxiliaryGesture(AuxiliaryGestureKind kind) {
    return _decide(
      command: InteractionCommand.auxiliaryUpdate,
      transitionEvent: InteractionEvent.auxiliaryGestureUpdate(kind: kind),
      expectedMode: InteractionMode.auxiliaryGesture,
    );
  }

  bool endAuxiliaryGesture() {
    return _decide(
      command: InteractionCommand.auxiliaryEnd,
      transitionEvent: InteractionEvent.auxiliaryGestureEnd,
      expectedMode: InteractionMode.idle,
    );
  }

  bool get _allowHorizontalPan => readGeometry().isPaginated;
  static const InteractionCore _core = InteractionCore();
  InteractionBlockReason? _lastAuxiliaryGestureBlockReason;
  InteractionBlockReason? _lastDndBlockReason;
  InteractionBlockReason? _lastDoubleTapDragBlockReason;
  InteractionBlockReason? _lastLongPressBlockReason;
  InteractionBlockReason? _lastPanBlockReason;
  InteractionBlockReason? _lastSelectionHandleBlockReason;
  InteractionBlockReason? _lastTableCellHandleBlockReason;
  InteractionBlockReason? _lastTapBlockReason;

  String? get debugAuxiliaryGestureBlockReason => _lastAuxiliaryGestureBlockReason?.name;
  String? get debugDndBlockReason => _lastDndBlockReason?.name;
  String? get debugDoubleTapDragBlockReason => _lastDoubleTapDragBlockReason?.name;
  String? get debugLongPressBlockReason => _lastLongPressBlockReason?.name;
  String? get debugPanBlockReason => _lastPanBlockReason?.name;
  String? get debugSelectionHandleBlockReason => _lastSelectionHandleBlockReason?.name;
  String? get debugTableCellHandleBlockReason => _lastTableCellHandleBlockReason?.name;
  String? get debugTapBlockReason => _lastTapBlockReason?.name;

  HorizontalScrollMetrics _resolveHorizontalMetrics() {
    final geometry = readGeometry();
    return resolveHorizontalScrollMetrics(
      controller: scope.horizontalScrollController,
      contentWidth: geometry.contentWidth,
      fallbackViewportDimension: readViewWidth(),
    );
  }

  double _resolvePointerX(double localX) {
    final geometry = readGeometry();
    final horizontalMetrics = _resolveHorizontalMetrics();
    final hScrollOffset = horizontalMetrics.scrollOffset;
    return geometry.toLogicalX(
      localX -
          geometry.contentStartX(
            viewportWidth: horizontalMetrics.viewportDimension,
            horizontalScrollOffset: hScrollOffset,
          ),
    );
  }

  SelectionHandleDragContext? _selectionHandleDragContext() {
    return _handleDragSession.selectionHandleDragContext();
  }

  Offset? _handleStemCenter(SelectionHandleInfo? handle, ContentGeometry geometry) {
    return _handleDragSession.getHandleStemCenter(
      handle,
      geometry,
      verticalScrollController: scope.verticalScrollController,
      resolveHorizontalMetrics: _resolveHorizontalMetrics,
    );
  }

  void _handleAutoScroll({required double y, required double x}) {
    _autoScrollSession.handle(
      y: y,
      x: x,
      viewWidth: readViewWidth(),
      viewHeight: readViewHeight(),
      handleDragPosition: handleDragPosition,
      longPressPosition: longPressPosition,
      dropPosition: dropPosition,
      verticalScrollController: scope.verticalScrollController,
      resolveHorizontalMetrics: _resolveHorizontalMetrics,
      getPageAtPosition: getPageAtPosition,
      getPointerX: _resolvePointerX,
      readDraggingHandleType: () => _handleDragSession.draggingHandleType,
      readDragAnchorHandle: () => _handleDragSession.dragAnchorHandle,
      readDoubleTapInitialRange: () => _handleDragSession.doubleTapInitialRange,
      dispatch: scope.controller.dispatch,
      scrollIntoView: scope.controller.scrollIntoView,
    );
  }

  RenderBox? _interactionRenderBox() {
    final keyed = interactionRegionKey.currentContext?.findRenderObject();
    if (keyed is RenderBox) {
      return keyed;
    }
    final fallback = context.findRenderObject();
    if (fallback is RenderBox) {
      return fallback;
    }
    return null;
  }

  Offset? viewportPositionFromGlobal(Offset globalPosition) {
    final renderBox = _interactionRenderBox();
    return renderBox?.globalToLocal(globalPosition);
  }

  ResolvedDragLocation? resolveSelectionDrag(Offset globalPosition) {
    if (interactionActive || _longPressSession.active || hasSelectionHandleDrag || pinchSession.isPinching) {
      return null;
    }

    final resolved = resolveDragLocation(globalPosition);
    if (resolved == null) {
      return null;
    }

    final isSelection = scope.editor.isSelectionHit(resolved.pageIdx, resolved.pointerX, resolved.localY);
    if (!isSelection) {
      return null;
    }

    return resolved;
  }

  ResolvedDragLocation? resolveDragLocation(Offset globalPosition) {
    final viewportPosition = viewportPositionFromGlobal(globalPosition);
    if (viewportPosition == null) {
      return null;
    }

    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
    final pointerX = _resolvePointerX(viewportPosition.dx);
    return (localPosition: viewportPosition, pageIdx: pageIdx, localY: localY, pointerX: pointerX);
  }

  bool shouldRejectLongPress(Offset globalPosition) {
    if (_handleDragSession.isCellHandleDragging || _doubleTapDragSession.active) {
      return false;
    }

    final viewportPosition = viewportPositionFromGlobal(globalPosition);
    if (viewportPosition == null) {
      return true;
    }

    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
    final pointerX = _resolvePointerX(viewportPosition.dx);
    return scope.editor.isSelectionHit(pageIdx, pointerX, localY);
  }

  InteractionRuntimeRead _runtimeRead() {
    final snapshot = scope.interactionState.snapshot();
    return InteractionRuntimeRead(
      snapshot: snapshot,
      pinchIsPinching: pinchSession.isPinching,
      pinchPointerCount: pinchSession.pointerCount,
      doubleTapActive: _doubleTapDragSession.active,
      doubleTapDragging: _doubleTapDragSession.dragging,
      tableCellHandleDragging: _handleDragSession.isCellHandleDragging,
      longPressActive: _longPressSession.active,
      hasPendingSelectionHandleDrag: _handleDragSession.hasPendingSelectionHandleDrag,
      hasAnyHandleDrag: _handleDragSession.hasAnyHandleDrag,
      panDragActive: _panSession.hasScrollDrag,
      dndLocked: snapshot.isDndActive,
    );
  }

  void _setBlockReason(InteractionCommandCategory category, InteractionBlockReason? reason) {
    switch (category) {
      case InteractionCommandCategory.tap:
        _lastTapBlockReason = reason;
      case InteractionCommandCategory.doubleTapDrag:
        _lastDoubleTapDragBlockReason = reason;
      case InteractionCommandCategory.longPress:
        _lastLongPressBlockReason = reason;
      case InteractionCommandCategory.pan:
        _lastPanBlockReason = reason;
      case InteractionCommandCategory.selectionHandle:
        _lastSelectionHandleBlockReason = reason;
      case InteractionCommandCategory.tableCellHandle:
        _lastTableCellHandleBlockReason = reason;
      case InteractionCommandCategory.dnd:
        _lastDndBlockReason = reason;
      case InteractionCommandCategory.auxiliaryGesture:
        _lastAuxiliaryGestureBlockReason = reason;
    }
  }

  void _reject(InteractionCommandCategory category, InteractionBlockReason reason) {
    _setBlockReason(category, reason);
  }

  bool _decide({
    required InteractionCommand command,
    InteractionEvent? transitionEvent,
    InteractionMode? expectedMode,
  }) {
    final decision = _core.decide(command: command, runtime: _runtimeRead());
    _setBlockReason(command.category, decision.reason);
    if (!decision.allowed) {
      return false;
    }

    if (transitionEvent != null) {
      final next = _applyTransition(transitionEvent);
      if (expectedMode != null && next.mode != expectedMode) {
        _setBlockReason(command.category, InteractionBlockReason.modeRejected);
        return false;
      }
    }

    _setBlockReason(command.category, null);
    return true;
  }

  InteractionSnapshot _applyTransition(InteractionEvent event) {
    final previous = scope.interactionState.snapshot();
    scope.interactionState.handle(event);
    final current = scope.interactionState.snapshot();

    final enteredPinch = !previous.isPinching && current.isPinching;
    final enteredAuxiliary = !previous.isAuxiliaryGesture && current.isAuxiliaryGesture;
    final exitedAutoScrollMode = _usesAutoScrollMode(previous.mode) && !_usesAutoScrollMode(current.mode);

    if (enteredPinch || enteredAuxiliary || exitedAutoScrollMode || event.type == InteractionEventType.pointerCancel) {
      _autoScrollSession.stop();
    }

    if (event.type == InteractionEventType.pointerCancel) {
      _tapSession.cancelTapTimer();
      _doubleTapDragSession.reset();
      _longPressSession.reset();
      _panSession.cancelDrag();
      _panResumeSession.reset();
      pinchSession.reset();
      _handleDragSession.clearSelectionHandleState();
      _dndSession.reset();
      longPressPosition.value = null;
      handleDragPosition.value = null;
    }

    if (enteredPinch || enteredAuxiliary) {
      showContextMenu.value = false;
    }

    return current;
  }

  bool _usesAutoScrollMode(InteractionMode mode) {
    switch (mode) {
      case InteractionMode.selectionHandleDragging:
      case InteractionMode.tableCellHandleDragging:
      case InteractionMode.longPressSelecting:
      case InteractionMode.doubleTapSelecting:
      case InteractionMode.dndLocal:
      case InteractionMode.dndExternal:
      case InteractionMode.auxiliaryGesture:
        return true;
      case InteractionMode.idle:
      case InteractionMode.panning:
      case InteractionMode.pinching:
        return false;
    }
  }

  void _recoverDndLockIfStale() {
    final snapshot = scope.interactionState.snapshot();
    if (!snapshot.isDndActive) {
      return;
    }

    if (snapshot.mode == InteractionMode.dndLocal && _dndSession.isNativeLocalDragActive) {
      return;
    }

    if (snapshot.mode == InteractionMode.dndExternal) {
      return;
    }

    endDndSession();
  }

  bool _consumeIfDndLocked({bool recover = false, VoidCallback? onLocked}) {
    if (!scope.interactionState.snapshot().isDndActive) {
      return false;
    }
    if (recover) {
      _recoverDndLockIfStale();
    }
    if (!scope.interactionState.snapshot().isDndActive) {
      return false;
    }
    onLocked?.call();
    return true;
  }

  void _clearResumedPanState() {
    _panResumeSession.reset();
  }
}
