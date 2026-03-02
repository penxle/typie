import 'dart:async';
import 'dart:math' as math;

import 'package:collection/collection.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/interaction/controller_state.dart';
import 'package:typie/screens/native_editor/view/interaction/gesture_controller.dart';
import 'package:typie/screens/native_editor/view/interaction/input.dart';
import 'package:typie/screens/native_editor/view/interaction/mode.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';
import 'package:typie/screens/native_editor/view/zoom_pinch.dart';

export 'package:typie/screens/native_editor/view/interaction/controller_state.dart'
    show ConditionalLongPressGestureRecognizer;

part 'controller_selection.dart';
part 'controller_pointer.dart';
part 'controller_dnd.dart';

typedef PagePositionResolver = (int pageIdx, double localY) Function(double y);
typedef ResolvedDragLocation = ({Offset localPosition, int pageIdx, double localY, double pointerX});

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

class EditorInteractionControllerDeps {
  const EditorInteractionControllerDeps({
    required this.context,
    required this.interactionRegionKey,
    required this.scope,
    required this.gesture,
    required this.pinch,
    required this.wheelZoomSession,
    required this.getPageAtPosition,
    required this.showContextMenu,
    required this.wasContextMenuOpen,
    required this.longPressPosition,
    required this.handleDragPosition,
    required this.dropPosition,
    required this.readViewWidth,
    required this.readViewHeight,
    required this.readGeometry,
  });

  final BuildContext context;
  final GlobalKey interactionRegionKey;
  final ContentScope scope;
  final GestureController gesture;
  final PinchGestureController pinch;
  final PinchZoomSession wheelZoomSession;
  final PagePositionResolver getPageAtPosition;
  final ValueNotifier<bool> showContextMenu;
  final ObjectRef<bool> wasContextMenuOpen;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<Offset?> dropPosition;
  final double Function() readViewWidth;
  final double Function() readViewHeight;
  final ContentGeometry Function() readGeometry;
}

class EditorInteractionController {
  EditorInteractionController({required EditorInteractionControllerDeps deps})
    : context = deps.context,
      interactionRegionKey = deps.interactionRegionKey,
      scope = deps.scope,
      gesture = deps.gesture,
      pinch = deps.pinch,
      wheelZoomSession = deps.wheelZoomSession,
      getPageAtPosition = deps.getPageAtPosition,
      showContextMenu = deps.showContextMenu,
      wasContextMenuOpen = deps.wasContextMenuOpen,
      longPressPosition = deps.longPressPosition,
      handleDragPosition = deps.handleDragPosition,
      dropPosition = deps.dropPosition,
      readViewWidth = deps.readViewWidth,
      readViewHeight = deps.readViewHeight,
      readGeometry = deps.readGeometry;

  final BuildContext context;
  final GlobalKey interactionRegionKey;
  final ContentScope scope;
  final GestureController gesture;
  final PinchGestureController pinch;
  final PinchZoomSession wheelZoomSession;
  final PagePositionResolver getPageAtPosition;
  final ValueNotifier<bool> showContextMenu;
  final ObjectRef<bool> wasContextMenuOpen;
  final ValueNotifier<Offset?> longPressPosition;
  final ValueNotifier<Offset?> handleDragPosition;
  final ValueNotifier<Offset?> dropPosition;
  final double Function() readViewWidth;
  final double Function() readViewHeight;
  final ContentGeometry Function() readGeometry;

  final _gestureState = ControllerSelectionGestureState();
  final _resumedPanState = ControllerResumedPanState();

  bool get interactionActive => _gestureState.active;
  bool get hasTextHandleDrag => gesture.hasTextHandleDrag;

  void startAuxiliaryGesture(AuxiliaryGestureKind kind) {
    _handleInteractionInput(AuxiliaryGestureStartInput(kind: kind));
  }

  void updateAuxiliaryGesture(AuxiliaryGestureKind kind) {
    _handleInteractionInput(AuxiliaryGestureUpdateInput(kind: kind));
  }

  void endAuxiliaryGesture() {
    _handleInteractionInput(const AuxiliaryGestureEndInput());
  }

  bool get _allowHorizontalPan => readGeometry().isPaginated;

  bool get _isSelecting => scope.interactionState.snapshot().isSelecting;

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
    if (interactionActive || hasTextHandleDrag || pinch.isPinching) {
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
    final pointerX = gesture.getPointerX(viewportPosition.dx);
    return (localPosition: viewportPosition, pageIdx: pageIdx, localY: localY, pointerX: pointerX);
  }

  bool shouldRejectLongPress(Offset globalPosition) {
    if (gesture.isCellHandleDragging || _gestureState.active) {
      return false;
    }

    final viewportPosition = viewportPositionFromGlobal(globalPosition);
    if (viewportPosition == null) {
      return true;
    }

    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
    final pointerX = gesture.getPointerX(viewportPosition.dx);
    return scope.editor.isSelectionHit(pageIdx, pointerX, localY);
  }

  void _handleInteractionInput(InteractionInput input) {
    final previous = scope.interactionState.snapshot();
    scope.interactionState.handle(input);
    final current = scope.interactionState.snapshot();

    final enteredPinch = !previous.isPinching && current.isPinching;
    final enteredAuxiliary = !previous.isAuxiliaryGesture && current.isAuxiliaryGesture;
    final exitedAutoScrollMode = _usesAutoScrollMode(previous.mode) && !_usesAutoScrollMode(current.mode);

    if (enteredPinch || enteredAuxiliary || exitedAutoScrollMode || input is PointerCancelInput) {
      gesture.stopAutoScroll();
    }

    if (enteredPinch || enteredAuxiliary) {
      showContextMenu.value = false;
    }
  }

  bool _usesAutoScrollMode(InteractionMode mode) {
    switch (mode) {
      case InteractionMode.textHandleDragging:
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

  bool _hasActiveDndLock() {
    return scope.interactionState.snapshot().isDndActive;
  }

  void _recoverDndLockIfStale() {
    if (!_hasActiveDndLock()) {
      return;
    }
    endDndSession();
  }

  bool _consumeIfDndLocked({bool recover = false, VoidCallback? onLocked}) {
    if (!_hasActiveDndLock()) {
      return false;
    }
    if (recover) {
      _recoverDndLockIfStale();
    }
    if (!_hasActiveDndLock()) {
      return false;
    }
    onLocked?.call();
    return true;
  }

  void _clearResumedPanState() {
    _resumedPanState.clear();
  }
}
