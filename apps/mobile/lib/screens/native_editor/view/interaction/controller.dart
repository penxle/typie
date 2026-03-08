import 'dart:async';
import 'dart:io';
import 'dart:math' as math;

import 'package:collection/collection.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/native/slate_reader.dart';
import 'package:typie/screens/native_editor/controller/dnd_controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/interaction/contracts.dart';
import 'package:typie/screens/native_editor/view/interaction/core.dart';
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

part 'gestures/dnd_gesture.dart';
part 'gestures/double_tap_drag_gesture.dart';
part 'gestures/long_press_gesture.dart';
part 'gestures/pan_gesture.dart';
part 'gestures/pinch_gesture.dart';
part 'gestures/pointer_gesture.dart';
part 'gestures/selection_handle_gesture.dart';
part 'gestures/table_handle_gesture.dart';
part 'gestures/tap_gesture.dart';
part 'semantics/auto_scroll_semantic.dart';
part 'semantics/cursor_move_semantic.dart';
part 'semantics/selection_expansion_semantic.dart';
part 'semantics/selection_handle_semantic.dart';
part 'utils.dart';

typedef PagePositionResolver = (int pageIdx, double localY) Function(double y);
typedef ResolvedDragLocation = ({Offset localPosition, int pageIdx, double localY, double pointerX});
typedef WordSelectionDragContext = ({SelectionHandleInfo anchor, Map<String, dynamic> initialRange});
typedef AutoScrollSelectionContext = ({
  SelectionHandleInfo? anchor,
  Map<String, dynamic>? initialRange,
  bool blockCursorFallback,
});

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

  final tapGesture = useMemoized(TapGesture.new);
  final doubleTapDragGesture = useMemoized(DoubleTapDragGesture.new);
  final longPressGesture = useMemoized(LongPressGesture.new);
  final panGesture = useMemoized(PanGesture.new);
  final panResumeGesture = useMemoized(PanResumeGesture.new);
  final pinchViewportGesture = useMemoized(PinchViewportGesture.new);
  final pinchGesture = useMemoized(() => PinchGesture(viewport: pinchViewportGesture), [pinchViewportGesture]);

  final cursorMoveSemantic = useMemoized(CursorMoveSemantic.new);
  final selectionExpansionSemantic = useMemoized(SelectionExpansionSemantic.new);
  final selectionHandleSemantic = useMemoized(SelectionHandleSemantic.new);
  final autoScrollSemantic = useMemoized(AutoScrollSemantic.new);

  final gestures = useMemoized(
    () => EditorInteractionGestures(
      tap: tapGesture,
      doubleTapDrag: doubleTapDragGesture,
      longPress: longPressGesture,
      pan: panGesture,
      panResume: panResumeGesture,
      pinch: pinchGesture,
      pinchViewport: pinchViewportGesture,
    ),
    [
      tapGesture,
      doubleTapDragGesture,
      longPressGesture,
      panGesture,
      panResumeGesture,
      pinchGesture,
      pinchViewportGesture,
    ],
  );

  final semantics = useMemoized(
    () => EditorInteractionSemantics(
      cursorMove: cursorMoveSemantic,
      selectionExpansion: selectionExpansionSemantic,
      selectionHandle: selectionHandleSemantic,
      autoScroll: autoScrollSemantic,
    ),
    [cursorMoveSemantic, selectionExpansionSemantic, selectionHandleSemantic, autoScrollSemantic],
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
      gestures: gestures,
      semantics: semantics,
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
      gestures,
      semantics,
      uiState,
    ],
  );

  useEffect(() {
    return () {
      gestures.reset();
      semantics.reset();
    };
  }, [gestures, semantics]);

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

class EditorInteractionGestures {
  const EditorInteractionGestures({
    required this.tap,
    required this.doubleTapDrag,
    required this.longPress,
    required this.pan,
    required this.panResume,
    required this.pinch,
    required this.pinchViewport,
  });

  final TapGesture tap;
  final DoubleTapDragGesture doubleTapDrag;
  final LongPressGesture longPress;
  final PanGesture pan;
  final PanResumeGesture panResume;
  final PinchGesture pinch;
  final PinchViewportGesture pinchViewport;

  void reset() {
    tap.reset();
    doubleTapDrag.reset();
    longPress.reset();
    pan.reset();
    panResume.reset();
    pinch.reset();
  }
}

class EditorInteractionSemantics {
  const EditorInteractionSemantics({
    required this.cursorMove,
    required this.selectionExpansion,
    required this.selectionHandle,
    required this.autoScroll,
  });

  final CursorMoveSemantic cursorMove;
  final SelectionExpansionSemantic selectionExpansion;
  final SelectionHandleSemantic selectionHandle;
  final AutoScrollSemantic autoScroll;

  void reset() {
    cursorMove.reset();
    selectionExpansion.reset();
    selectionHandle.reset();
    autoScroll.reset();
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
    required EditorInteractionGestures gestures,
    required EditorInteractionSemantics semantics,
    required this.getPageAtPosition,
    required EditorInteractionUiState ui,
    required this.readViewWidth,
    required this.readViewHeight,
    required this.readGeometry,
  }) : _tapGesture = gestures.tap,
       _doubleTapDragGesture = gestures.doubleTapDrag,
       _longPressGesture = gestures.longPress,
       _panGesture = gestures.pan,
       _panResumeGesture = gestures.panResume,
       pinchGesture = gestures.pinch,
       pinchViewportGesture = gestures.pinchViewport,
       _selectionExpansionSemantic = semantics.selectionExpansion,
       _selectionHandleSemantic = semantics.selectionHandle,
       _autoScrollSemantic = semantics.autoScroll,
       showContextMenu = ui.showContextMenu,
       wasContextMenuOpen = ui.wasContextMenuOpen,
       longPressPosition = ui.longPressPosition,
       handleDragPosition = ui.handleDragPosition,
       dropPosition = ui.dropPosition;

  final BuildContext context;
  final GlobalKey interactionRegionKey;
  final ContentScope scope;

  final TapGesture _tapGesture;
  final DoubleTapDragGesture _doubleTapDragGesture;
  final LongPressGesture _longPressGesture;
  final PanGesture _panGesture;
  final PanResumeGesture _panResumeGesture;
  final PinchGesture pinchGesture;
  final PinchViewportGesture pinchViewportGesture;

  final SelectionExpansionSemantic _selectionExpansionSemantic;
  final SelectionHandleSemantic _selectionHandleSemantic;
  final AutoScrollSemantic _autoScrollSemantic;
  final PagePositionResolver getPageAtPosition;
  final ValueNotifier<bool> showContextMenu;
  final ObjectRef<bool> wasContextMenuOpen;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<Offset?> dropPosition;
  final double Function() readViewWidth;
  final double Function() readViewHeight;
  final ContentGeometry Function() readGeometry;
  @visibleForTesting
  static bool? debugIsAndroidOverride;

  InteractionMode get _interactionMode => scope.interactionState.snapshot().mode;
  bool get _isSelectionHandleMode => _interactionMode == InteractionMode.selectionHandleDragging;
  bool get _isTableCellHandleMode => _interactionMode == InteractionMode.tableCellHandleDragging;
  bool get isDoubleTapDragActive => _doubleTapDragGesture.active;
  bool get hasSelectionHandleDrag => _isSelectionHandleMode || _selectionHandleSemantic.hasSelectionHandleDrag;
  bool get isTableCellHandleDragging => _isTableCellHandleMode;
  bool get _isAndroid => debugIsAndroidOverride ?? Platform.isAndroid;

  void clearTapHistory() {
    _tapGesture
      ..clearTapHistory()
      ..skipNextContextMenu = true;
  }

  Offset? selectionHandleViewportPosition(SelectionHandleInfo? handle, ContentGeometry geometry) {
    return _selectionHandleSemantic.getHandlePosition(
      handle,
      geometry,
      verticalScrollController: scope.verticalScrollController,
      resolveHorizontalMetrics: _resolveHorizontalMetrics,
    );
  }

  void stopSelectionHandlesAndAutoScroll() {
    _selectionHandleSemantic.clearSelectionHandleState();
    if (scope.interactionState.snapshot().mode != InteractionMode.longPressWordSelecting) {
      _selectionExpansionSemantic.clear();
    }
    _autoScrollSemantic.stop();
  }

  void stopInteractionAutoScroll() {
    _autoScrollSemantic.stop();
  }

  void cancelInteractionScrollDrag() {
    _panGesture.cancelDrag();
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
    return _selectionHandleSemantic.selectionHandleDragContext();
  }

  Offset? _handleStemCenter(SelectionHandleInfo? handle, ContentGeometry geometry) {
    return _selectionHandleSemantic.getHandleStemCenter(
      handle,
      geometry,
      verticalScrollController: scope.verticalScrollController,
      resolveHorizontalMetrics: _resolveHorizontalMetrics,
    );
  }

  void _handleAutoScroll({required double y, required double x}) {
    _autoScrollSemantic.handle(
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
      readSelectionContext: _resolveAutoScrollSelectionContext,
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
    final snapshot = scope.interactionState.snapshot();
    final blockedBySelectionMode = snapshot.isSelecting;
    final blockedByPendingDoubleTap = _doubleTapDragGesture.pending;
    if (blockedBySelectionMode || blockedByPendingDoubleTap || pinchGesture.isPinching) {
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

  ({Offset viewportPosition, _LongPressSemanticIntent semanticIntent})? _resolveLongPressAdmission({
    required Offset? viewportPosition,
    required InteractionRuntimeRead runtime,
  }) {
    if (!_core.decide(
      command: InteractionCommand.longPressStart(viewportPosition: viewportPosition),
      runtime: runtime,
    )) {
      return null;
    }
    if (!_core.decide(command: InteractionCommand.longPressBeginSelecting, runtime: runtime)) {
      return null;
    }
    if (_longPressGesture.active || viewportPosition == null) {
      return null;
    }

    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
    final pointerX = _resolvePointerX(viewportPosition.dx);
    final selectionHit = scope.editor.isSelectionHit(pageIdx, pointerX, localY);
    if (selectionHit) {
      if (!_isAndroid) {
        return null;
      }
      final isCollapsed = scope.controller.state.selection?.collapsed ?? true;
      if (!isCollapsed) {
        return null;
      }
    }

    final semanticIntent = _resolveLongPressSemanticIntent(viewportPosition);
    return (viewportPosition: viewportPosition, semanticIntent: semanticIntent);
  }

  bool shouldRejectLongPress(Offset globalPosition) {
    final viewportPosition = viewportPositionFromGlobal(globalPosition);
    return _resolveLongPressAdmission(viewportPosition: viewportPosition, runtime: _runtimeRead()) == null;
  }

  WordSelectionDragContext? _resolveWordSelectionDragContext() {
    final cached = _selectionExpansionSemantic.context;
    if (cached != null) {
      return cached;
    }

    final selection = scope.controller.state.selection;
    if (selection == null || selection.collapsed) {
      return null;
    }

    final anchor = selection.fromBounds;
    if (anchor == null) {
      return null;
    }

    final initialRange = selection.range;
    _selectionExpansionSemantic.set(anchor: anchor, initialRange: initialRange);
    return _selectionExpansionSemantic.context;
  }

  WordSelectionDragContext? _resolveLongPressWordSelectionContext() {
    if (scope.interactionState.snapshot().mode != InteractionMode.longPressWordSelecting) {
      return null;
    }

    final cached = _selectionExpansionSemantic.context;
    if (cached != null) {
      return cached;
    }

    _selectionExpansionSemantic.adoptWordSelection(scope.controller.state.selection);
    return _selectionExpansionSemantic.context;
  }

  AutoScrollSelectionContext _resolveAutoScrollSelectionContext() {
    final interactionMode = scope.interactionState.snapshot().mode;

    if (interactionMode == InteractionMode.doubleTapSelecting) {
      final doubleTapContext = _resolveWordSelectionDragContext();
      if (doubleTapContext != null) {
        return (
          anchor: doubleTapContext.anchor,
          initialRange: doubleTapContext.initialRange,
          blockCursorFallback: true,
        );
      }
      return (anchor: null, initialRange: null, blockCursorFallback: true);
    }

    final draggingHandleType = _selectionHandleSemantic.draggingHandleType;
    final dragAnchorHandle = _selectionHandleSemantic.dragAnchorHandle;
    if (interactionMode == InteractionMode.selectionHandleDragging || draggingHandleType != null) {
      return (anchor: dragAnchorHandle, initialRange: null, blockCursorFallback: true);
    }

    if (interactionMode == InteractionMode.longPressWordSelecting) {
      final longPressWordSelectionContext = _resolveLongPressWordSelectionContext();
      if (longPressWordSelectionContext != null) {
        return (
          anchor: longPressWordSelectionContext.anchor,
          initialRange: longPressWordSelectionContext.initialRange,
          blockCursorFallback: true,
        );
      }
      return (anchor: null, initialRange: null, blockCursorFallback: true);
    }

    return (anchor: null, initialRange: null, blockCursorFallback: false);
  }

  InteractionRuntimeRead _runtimeRead() {
    final snapshot = scope.interactionState.snapshot();
    return InteractionRuntimeRead(
      snapshot: snapshot,
      pinchIsPinching: pinchGesture.isPinching,
      pinchPointerCount: pinchGesture.pointerCount,
      doubleTapActive: _doubleTapDragGesture.active,
      doubleTapDragging: _doubleTapDragGesture.dragging,
      tableCellHandleDragging: snapshot.mode == InteractionMode.tableCellHandleDragging,
      hasPendingSelectionHandleDrag: _selectionHandleSemantic.hasPendingSelectionHandleDrag,
      hasAnyHandleDrag:
          snapshot.mode == InteractionMode.selectionHandleDragging ||
          snapshot.mode == InteractionMode.tableCellHandleDragging ||
          _selectionHandleSemantic.hasAnyHandleDrag,
      panDragActive: _panGesture.hasScrollDrag,
    );
  }

  bool _decide({
    required InteractionCommand command,
    InteractionEvent? transitionEvent,
    InteractionMode? expectedMode,
  }) {
    if (!_core.decide(command: command, runtime: _runtimeRead())) {
      return false;
    }

    if (transitionEvent != null) {
      final next = _applyTransition(transitionEvent);
      if (expectedMode != null && next.mode != expectedMode) {
        return false;
      }
    }
    return true;
  }

  InteractionSnapshot _applyTransition(InteractionEvent event) {
    final previous = scope.interactionState.snapshot();
    scope.interactionState.handle(event);
    final current = scope.interactionState.snapshot();

    _cleanupForModeTransition(previousMode: previous.mode, currentMode: current.mode);

    if (event.type == InteractionEventType.pointerCancel) {
      _tapGesture.cancelTapTimer();
      _doubleTapDragGesture.reset();
      _longPressGesture.reset();
      _selectionExpansionSemantic.reset();
      _panGesture.cancelDrag();
      _panResumeGesture.reset();
      pinchGesture.reset();
      _selectionHandleSemantic.clearSelectionHandleState();
      longPressPosition.value = null;
      handleDragPosition.value = null;
    }

    return current;
  }

  void _cleanupForModeTransition({required InteractionMode previousMode, required InteractionMode currentMode}) {
    if (previousMode == currentMode) {
      return;
    }

    if (_usesAutoScrollMode(previousMode) && !_usesAutoScrollMode(currentMode)) {
      _autoScrollSemantic.stop();
    }

    if (previousMode == InteractionMode.longPressSelecting || previousMode == InteractionMode.longPressWordSelecting) {
      _clearLongPressState();
      _selectionExpansionSemantic.clear();
    }

    switch (currentMode) {
      case InteractionMode.pinching:
        _tapGesture.cancelTapTimer();
        _doubleTapDragGesture.stop();
        _clearSelectionExpansionState();
        _panGesture.cancelDrag();
        _panResumeGesture.reset();
        _clearLongPressState();
        _dismissSelectionUi();
        return;
      case InteractionMode.longPressSelecting:
        _tapGesture.cancelTapTimer();
        _clearSelectionExpansionState();
        _dismissSelectionUi();
        return;
      case InteractionMode.longPressWordSelecting:
        _tapGesture.cancelTapTimer();
        _clearHandleDragState();
        _dismissSelectionUi();
        return;
      case InteractionMode.doubleTapSelecting:
        _clearSelectionExpansionState();
        _clearLongPressState();
        _dismissSelectionUi();
        return;
      case InteractionMode.selectionHandleDragging:
      case InteractionMode.tableCellHandleDragging:
        _clearLongPressState();
        _selectionExpansionSemantic.clear();
        _dismissSelectionUi();
        return;
      case InteractionMode.dndLocal:
      case InteractionMode.dndExternal:
        _dismissSelectionUi();
        return;
      case InteractionMode.auxiliaryGesture:
        _dismissSelectionUi();
        return;
      case InteractionMode.idle:
      case InteractionMode.panning:
        return;
    }
  }

  void _clearSelectionExpansionState() {
    _clearHandleDragState();
    _selectionExpansionSemantic.clear();
  }

  void _clearHandleDragState() {
    _selectionHandleSemantic.clearSelectionHandleState();
    handleDragPosition.value = null;
  }

  void _clearLongPressState() {
    _longPressGesture.end();
    longPressPosition.value = null;
  }

  void _dismissSelectionUi() {
    showContextMenu.value = false;
    _autoScrollSemantic.stop();
  }

  bool _usesAutoScrollMode(InteractionMode mode) {
    switch (mode) {
      case InteractionMode.selectionHandleDragging:
      case InteractionMode.tableCellHandleDragging:
      case InteractionMode.longPressSelecting:
      case InteractionMode.longPressWordSelecting:
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

  bool _consumeIfDndLocked({VoidCallback? onLocked}) {
    if (!scope.interactionState.snapshot().isDndActive) {
      return false;
    }
    onLocked?.call();
    return true;
  }

  void _clearResumedPanState() {
    _panResumeGesture.reset();
  }
}
