import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/context_menu.dart';
import 'package:typie/screens/native_editor/view/editor_draggable.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/gesture.dart';
import 'package:typie/screens/native_editor/view/page.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/selection.dart';
import 'package:typie/screens/native_editor/view/table_overlay.dart';
import 'package:typie/screens/native_editor/view/title.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';
import 'package:typie/screens/native_editor/view/zoom_pinch.dart';
import 'package:typie/services/preference.dart';

class PageList extends HookWidget {
  const PageList({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final pref = useService<Pref>();
    final state = useListenable(scope.controller);

    final pages = state.state.pages;
    final cursor = state.state.cursor;
    final renderedCursor = useValueListenable(scope.presentedViewport).cursor;
    final isFocused = state.state.isFocused;
    final isSelecting = state.state.isSelecting;
    final selection = state.state.selection;
    final fromHandle = state.state.selection?.fromBounds;
    final toHandle = state.state.selection?.toBounds;
    final dropIndicator = state.state.dropIndicator;
    final isDropping = useValueListenable(scope.dndController.isDropping);
    final tableOverlays = useValueListenable(scope.controller.tableOverlays);
    final isTableCellSelectorSelection = tableOverlays.any((overlay) => overlay.isFocused && overlay.showCellSelector);

    final verticalScrollController = scope.verticalScrollController;
    final horizontalScrollController = scope.horizontalScrollController;
    final handleMetricsRevision = useValueNotifier(0);

    useValueListenable(scope.titleAreaHeight);
    final zoom = useValueListenable(scope.displayZoom);

    (int pageIdx, double localY) getPageAtPosition(double y) {
      final geo = scope.geometry;
      final offsets = geo.computeCumulativePageOffsets();
      final scrollOffset = resolveScrollOffset(scope.verticalScrollController);
      final absoluteY = y + scrollOffset;
      final extensionAreaTop = (geo.titleAreaHeight - geo.toDisplayY(ContentGeometry.pagePadding)).clamp(
        0.0,
        double.infinity,
      );

      if (absoluteY < extensionAreaTop) {
        return (-1, absoluteY);
      }

      final adjustedY = absoluteY - geo.titleAreaHeight;

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

      final pageIdx = (low - 1).clamp(0, geo.pages.length - 1);
      final localY = geo.toLogicalY(adjustedY - offsets[pageIdx]);
      return (pageIdx, localY);
    }

    final showContextMenu = useState(false);
    final wasContextMenuOpen = useRef(false);
    final clipboard = useMemoized(EditorClipboard.new);
    final viewportWidth = useValueNotifier<double>(0);

    HorizontalScrollMetrics resolveHorizontalMetrics() {
      final geo = scope.geometry;
      return resolveHorizontalScrollMetrics(
        controller: horizontalScrollController,
        contentWidth: geo.contentWidth,
        fallbackViewportDimension: viewportWidth.value,
      );
    }

    final longPressPosition = scope.longPressPosition;
    final handleDragPosition = scope.handleDragPosition;
    final dropPosition = useValueNotifier<Offset?>(null);
    final previousDropIndicatorKey = useRef<String?>(null);

    String? dropIndicatorKey(DropIndicatorInfo? info) {
      if (info == null) {
        return null;
      }
      return '${info.pageIdx}:${info.x}:${info.y}:${info.width}:${info.height}';
    }

    final gesture = useMemoized(
      () => GestureController(
        verticalScrollController: verticalScrollController,
        horizontalScrollController: horizontalScrollController,
        controller: scope.controller,
        getPageAtPosition: getPageAtPosition,
        getPointerX: (localX) {
          final geo = scope.geometry;
          final horizontalMetrics = resolveHorizontalMetrics();
          final hScrollOffset = horizontalMetrics.scrollOffset;
          return geo.toLogicalX(
            localX -
                geo.contentStartX(
                  viewportWidth: horizontalMetrics.viewportDimension,
                  horizontalScrollOffset: hScrollOffset,
                ),
          );
        },
        getHorizontalMetrics: resolveHorizontalMetrics,
        isLongPressing: scope.isLongPressing,
      ),
    );
    final wheelZoomSession = useMemoized(PinchZoomSession.new);
    final gestureState = gesture.state;
    final pinch = useMemoized(PinchGestureController.new);
    final resumedPanPointer = useRef<int?>(null);
    final resumedPanLastLocalPosition = useRef<Offset?>(null);
    final resumedPanActive = useRef(false);

    useEffect(() => gesture.dispose, const []);
    useEffect(() => pinch.reset, [pinch]);

    useEffect(() {
      void onScroll() {
        if (showContextMenu.value) {
          showContextMenu.value = false;
        }
      }

      verticalScrollController.addListener(onScroll);
      horizontalScrollController.addListener(onScroll);
      return () {
        verticalScrollController.removeListener(onScroll);
        horizontalScrollController.removeListener(onScroll);
      };
    }, [verticalScrollController, horizontalScrollController]);

    final prevFromHandle = useRef<SelectionHandleInfo?>(null);
    final prevToHandle = useRef<SelectionHandleInfo?>(null);
    final prevSelectionRangeKey = useRef<String?>(null);
    final wasSelecting = useRef(false);

    bool didHandlePositionChange(SelectionHandleInfo? previous, SelectionHandleInfo? current) {
      if (previous == null || current == null) {
        return false;
      }

      return previous.pageIdx != current.pageIdx || previous.x != current.x || previous.y != current.y;
    }

    String? selectionRangeKey(EditorSelection? value) {
      if (value == null) {
        return null;
      }

      final anchor = value.range['anchor'] as Map<String, dynamic>?;
      final head = value.range['head'] as Map<String, dynamic>?;
      if (anchor == null || head == null) {
        return null;
      }

      return '${anchor['nodeId']}:${anchor['offset']}:${anchor['affinity']}'
          '|${head['nodeId']}:${head['offset']}:${head['affinity']}'
          '|${value.collapsed ? 1 : 0}';
    }

    useEffect(() {
      final isLongPressing = scope.isLongPressing.value;
      final previousSelectionRangeKey = prevSelectionRangeKey.value;
      final currentSelectionRangeKey = selectionRangeKey(selection);
      final isCollapsed = fromHandle == null || toHandle == null;
      final handleDragCanceledByTableSelection =
          isTableCellSelectorSelection && !gesture.isCellHandleDragging && gesture.hasTextHandleDrag;
      final justFinishedSelecting = wasSelecting.value && !isSelecting;
      final selectionChanged = fromHandle != prevFromHandle.value || toHandle != prevToHandle.value;
      final hasPreviousSelectionSnapshot = previousSelectionRangeKey != null;
      final handlesJustAppeared =
          hasPreviousSelectionSnapshot &&
          prevFromHandle.value == null &&
          prevToHandle.value == null &&
          fromHandle != null &&
          toHandle != null;
      final anyHandleMoved =
          didHandlePositionChange(prevFromHandle.value, fromHandle) ||
          didHandlePositionChange(prevToHandle.value, toHandle);
      final longPressSelectionMoved =
          isLongPressing &&
          previousSelectionRangeKey != null &&
          currentSelectionRangeKey != null &&
          previousSelectionRangeKey != currentSelectionRangeKey &&
          !handlesJustAppeared;

      final shouldResetTextHandleDrag =
          !gesture.isCellHandleDragging && (isCollapsed || handleDragCanceledByTableSelection);

      if (handlesJustAppeared) {
        unawaited(HapticFeedback.selectionClick());
      }

      if (!gesture.isCellHandleDragging && gesture.hasTextHandleDrag && anyHandleMoved) {
        unawaited(HapticFeedback.selectionClick());
      }

      if (longPressSelectionMoved) {
        unawaited(HapticFeedback.selectionClick());
      }

      if (shouldResetTextHandleDrag) {
        gesture.stopSelectionHandlesAndAutoScroll();
        showContextMenu.value = false;
      } else if (!isSelecting && !gestureState.active) {
        if (justFinishedSelecting) {
          showContextMenu.value = true;
        } else if (selectionChanged) {
          showContextMenu.value = true;
        }
      }

      if (isSelecting || gestureState.active) {
        showContextMenu.value = false;
      }

      wasSelecting.value = isSelecting;
      prevFromHandle.value = fromHandle;
      prevToHandle.value = toHandle;
      prevSelectionRangeKey.value = currentSelectionRangeKey;
      return null;
    }, [selection, fromHandle, toHandle, isSelecting, isTableCellSelectorSelection, scope.isLongPressing.value]);

    useEffect(() {
      final nextKey = dropIndicatorKey(dropIndicator);
      final previousKey = previousDropIndicatorKey.value;
      if (isDropping && previousKey != null && nextKey != null && previousKey != nextKey) {
        unawaited(HapticFeedback.selectionClick());
      }
      previousDropIndicatorKey.value = nextKey;
      return null;
    }, [dropIndicator, isDropping]);

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewWidth = constraints.maxWidth;
        final viewHeight = constraints.maxHeight;
        final geo = scope.geometry;
        if (viewportWidth.value != viewWidth) {
          final nextViewportWidth = viewWidth;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (viewportWidth.value != nextViewportWidth) {
              viewportWidth.value = nextViewportWidth;
            }
          });
        }

        Offset? viewportPositionFromGlobal(Offset globalPosition) {
          final renderBox = context.findRenderObject() as RenderBox?;
          return renderBox?.globalToLocal(globalPosition);
        }

        bool isConsecutiveTap({required Offset localPosition, required DateTime now}) {
          return gesture.isConsecutiveTap(localPosition: localPosition, now: now);
        }

        String? zoomSnapKey(double value) {
          final layout = scope.controller.state.layout;
          if (layout is! PaginatedLayout || viewWidth <= 0) {
            return null;
          }

          final fitWidthZoom = computePaginatedFitWidthZoom(pageWidth: layout.pageWidth, viewportWidth: viewWidth);
          final unitZoom = clampDocumentZoom(1, bounds: computePaginatedZoomBounds(pageWidth: layout.pageWidth));

          if (zoomEquals(value, fitWidthZoom)) {
            return 'fit-width';
          }
          if (zoomEquals(value, unitZoom)) {
            return 'unit';
          }
          return null;
        }

        void maybeSendZoomSnapHaptic({required double previousZoom, required double nextZoom}) {
          if (zoomEquals(previousZoom, nextZoom)) {
            return;
          }

          final nextSnap = zoomSnapKey(nextZoom);
          if (nextSnap == null) {
            return;
          }

          final previousSnap = zoomSnapKey(previousZoom);
          if (previousSnap == nextSnap) {
            return;
          }

          unawaited(HapticFeedback.selectionClick());
        }

        void beginPinchIfNeeded() {
          final started = pinch.beginIfNeeded(
            isPaginated: geo.isPaginated,
            currentZoom: zoom,
            resolveLogicalX: gesture.getPointerX,
            resolvePageAtPosition: getPageAtPosition,
          );
          if (!started) {
            return;
          }

          resumedPanPointer.value = null;
          resumedPanLastLocalPosition.value = null;
          resumedPanActive.value = false;

          gesture
            ..cancelTapTimer()
            ..stopSelectionHandlesAndAutoScroll()
            ..cancelScrollDrag();
          gestureState.stop();
          longPressPosition.value = null;
          handleDragPosition.value = null;
          showContextMenu.value = false;
          if (scope.controller.state.isSelecting) {
            scope.controller.setSelecting(false);
          }
        }

        void updatePinchZoom() {
          pinch.updateIfNeeded(
            isPaginated: geo.isPaginated,
            layout: scope.controller.state.layout,
            viewportWidth: viewWidth,
            currentZoom: zoom,
            resolveLogicalX: gesture.getPointerX,
            resolvePageAtPosition: getPageAtPosition,
            setZoom: scope.setZoom,
            geometryBuilder: (nextZoom) => ContentGeometry(
              layout: scope.controller.state.layout!,
              pages: scope.controller.state.pages,
              titleAreaHeight: scope.titleAreaHeight.value,
              selection: scope.controller.state.selection,
              zoom: nextZoom,
            ),
            horizontalScrollController: horizontalScrollController,
            verticalScrollController: verticalScrollController,
            isMounted: () => context.mounted,
            onZoomChanged: (previousZoom, nextZoom) {
              maybeSendZoomSnapHaptic(previousZoom: previousZoom, nextZoom: nextZoom);
            },
          );
        }

        void endPinchIfNeeded() {
          pinch.endIfNeeded(currentZoom: scope.displayZoom.value, setZoom: scope.setZoom);
        }

        void endTextHandleDrag() {
          if (pinch.isPinching) {
            return;
          }
          if (gesture.isCellHandleDragging) {
            return;
          }
          final hadHandleDrag = gesture.hasTextHandleDrag || handleDragPosition.value != null;
          if (!hadHandleDrag) {
            return;
          }
          gesture.stopSelectionHandlesAndAutoScroll();
          handleDragPosition.value = null;
          if (scope.controller.state.isSelecting) {
            scope.controller.setSelecting(false);
          }
        }

        bool dispatchDoubleTapSelection(Offset localPosition) {
          if (pinch.isPinching) {
            return false;
          }
          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
          if (pageIdx < 0) {
            return false;
          }

          showContextMenu.value = false;
          scope.inputController.commitComposing();
          scope.inputController.openInput();

          final pointerX = gesture.getPointerX(localPosition.dx);
          scope.controller.dispatch({
            'type': 'pointerDown',
            'pageIdx': pageIdx,
            'x': pointerX,
            'y': localY,
            'clickCount': 2,
            'button': 'primary',
            'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
          });
          scope.controller.dispatch({
            'type': 'pointerUp',
            'pageIdx': pageIdx,
            'x': pointerX,
            'y': localY,
            'button': 'primary',
            'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
          });

          gesture.clearTapHistory();

          scope.controller.scrollIntoView();
          return true;
        }

        void prepareDoubleTapDrag(Offset localPosition) {
          if (pinch.isPinching) {
            return;
          }
          gesture
            ..cancelTapTimer()
            ..setTapDispatched(true)
            ..clearSelectionHandleState()
            ..stopAutoScroll();

          longPressPosition.value = null;
          showContextMenu.value = false;
          gestureState.prepare(localPosition);
          handleDragPosition.value = null;
        }

        void startDoubleTapDrag(Offset localPosition) {
          if (pinch.isPinching) {
            return;
          }
          gesture
            ..cancelTapTimer()
            ..setTapDispatched(true)
            ..setTextHandleDragType(SelectionHandleType.to)
            ..stopAutoScroll();

          longPressPosition.value = null;
          showContextMenu.value = false;
          gestureState.begin(localPosition);
          handleDragPosition.value = null;
          scope.controller.setSelecting(true);
        }

        void endDoubleTapDrag() {
          gestureState.stop();
          endTextHandleDrag();
        }

        ({SelectionHandleInfo anchor, Map<String, dynamic> initialRange})? resolveDoubleTapDragSelectionContext() {
          final dragAnchorHandle = gesture.dragAnchorHandle;
          final doubleTapInitialRange = gesture.doubleTapInitialRange;
          if (dragAnchorHandle != null && doubleTapInitialRange != null) {
            return (anchor: dragAnchorHandle, initialRange: doubleTapInitialRange);
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
          gesture
            ..setDragAnchorHandle(anchor)
            ..setDoubleTapInitialRange(initialRange);
          return (anchor: anchor, initialRange: initialRange);
        }

        void updateDoubleTapDragSelection(Offset localPosition) {
          if (pinch.isPinching) {
            return;
          }
          if (!gestureState.dragging) {
            return;
          }

          final startPosition = gestureState.start;
          if (startPosition != null && (localPosition - startPosition).distance < 4) {
            return;
          }

          handleDragPosition.value = localPosition;
          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
          final pointerX = gesture.getPointerX(localPosition.dx);
          final context = resolveDoubleTapDragSelectionContext();
          if (context != null && pageIdx >= 0) {
            scope.controller.dispatch({
              'type': 'extendSelectionTo',
              'anchorPageIdx': context.anchor.pageIdx,
              'anchorX': context.anchor.x,
              'anchorY': context.anchor.y + context.anchor.height / 2,
              'headPageIdx': pageIdx,
              'headX': pointerX,
              'headY': localY,
              'doubleTapInitialRange': context.initialRange,
            });
            gesture.handleAutoScroll(
              y: localPosition.dy,
              x: localPosition.dx,
              viewWidth: viewWidth,
              viewHeight: viewHeight,
              handleDragPosition: handleDragPosition,
              longPressPosition: longPressPosition,
              dropPosition: dropPosition,
            );
          }
        }

        void onHandleDragDown(SelectionHandleType type, DragDownDetails details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          gesture.rememberPointerDown(renderBox.globalToLocal(details.globalPosition));
        }

        void onHandleDragStart(SelectionHandleType type, DragStartDetails details) {
          if (pinch.isPinching) {
            return;
          }
          scope.controller.setSelecting(true);

          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = gesture.pointerDownTouchPosition() ?? renderBox.globalToLocal(details.globalPosition);
          final handle = type == SelectionHandleType.from ? fromHandle : toHandle;
          gesture.beginTextHandleDrag(
            type: type,
            touchPosition: touchPosition,
            handleScreenPosition: gesture.getHandleStemCenter(handle, scope.geometry) ?? touchPosition,
            anchorHandle: type == SelectionHandleType.from ? toHandle : fromHandle,
          );
        }

        void onHandleDragUpdate(SelectionHandleType type, DragUpdateDetails details) {
          if (pinch.isPinching) {
            return;
          }
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = renderBox.globalToLocal(details.globalPosition);
          final dragContext = gesture.selectionHandleDragContext();
          if (dragContext == null) {
            return;
          }

          final delta = touchPosition - dragContext.startTouchPosition;
          final selectionScreenPosition = dragContext.startHandleScreenPosition + delta;

          handleDragPosition.value = selectionScreenPosition;

          final (pageIdx, localY) = getPageAtPosition(selectionScreenPosition.dy);
          if (pageIdx >= 0) {
            final pointerX = gesture.getPointerX(selectionScreenPosition.dx);
            scope.controller.dispatch({
              'type': 'extendSelectionTo',
              'anchorPageIdx': dragContext.anchorHandle.pageIdx,
              'anchorX': dragContext.anchorHandle.x,
              'anchorY': dragContext.anchorHandle.y + dragContext.anchorHandle.height / 2,
              'headPageIdx': pageIdx,
              'headX': pointerX,
              'headY': localY,
            });
          }

          gesture.handleAutoScroll(
            y: touchPosition.dy,
            x: touchPosition.dx,
            viewWidth: viewWidth,
            viewHeight: viewHeight,
            handleDragPosition: handleDragPosition,
            longPressPosition: longPressPosition,
            dropPosition: dropPosition,
          );
        }

        void onHandleDragEnd(SelectionHandleType type, DragEndDetails details) {
          endTextHandleDrag();
        }

        final offsets = geo.computeCumulativePageOffsets();
        final contentWidth = geo.contentWidth;
        final allowHorizontalPan = geo.isPaginated;
        final hasRangeSelection = !(state.state.selection?.collapsed ?? true);
        final horizontalPhysics = isSelecting || !allowHorizontalPan
            ? const NeverScrollableScrollPhysics()
            : const _NonGestureBouncingScrollPhysics();

        final contentBottomPadding = geo.bottomPadding(
          viewportHeight: resolveScrollPosition(verticalScrollController)?.viewportDimension ?? viewHeight,
          cursor: cursor,
          typewriterEnabled: pref.typewriterEnabled,
          typewriterPosition: pref.typewriterPosition,
        );

        void clearResumedPanState() {
          resumedPanPointer.value = null;
          resumedPanLastLocalPosition.value = null;
          resumedPanActive.value = false;
        }

        bool handlePointerZoom(PointerScrollEvent event, Set<LogicalKeyboardKey> keysPressed) {
          if (!geo.isPaginated) {
            return false;
          }

          final isZoomModifierPressed =
              keysPressed.contains(LogicalKeyboardKey.controlLeft) ||
              keysPressed.contains(LogicalKeyboardKey.controlRight) ||
              keysPressed.contains(LogicalKeyboardKey.metaLeft) ||
              keysPressed.contains(LogicalKeyboardKey.metaRight);
          if (!isZoomModifierPressed) {
            return false;
          }

          final layout = scope.controller.state.layout;
          if (layout is! PaginatedLayout) {
            return false;
          }

          final zoomDelta = event.scrollDelta.dy.abs() >= event.scrollDelta.dx.abs()
              ? event.scrollDelta.dy
              : event.scrollDelta.dx;
          if (zoomDelta == 0) {
            return true;
          }

          final nextZoom = clampPaginatedZoom(
            zoom: zoom * math.exp(-zoomDelta / 240),
            pageWidth: layout.pageWidth,
            viewportWidth: viewWidth,
          );
          if (zoomEquals(nextZoom, zoom)) {
            return true;
          }

          final focal = event.localPosition;
          final logicalX = gesture.getPointerX(focal.dx);
          final (pageIdx, logicalY) = getPageAtPosition(focal.dy);
          wheelZoomSession.captureAnchor(pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);

          maybeSendZoomSnapHaptic(previousZoom: zoom, nextZoom: nextZoom);
          scope.setZoom(nextZoom);
          wheelZoomSession.syncViewport(
            focal: focal,
            geometry: ContentGeometry(
              layout: layout,
              pages: scope.controller.state.pages,
              titleAreaHeight: scope.titleAreaHeight.value,
              selection: scope.controller.state.selection,
              zoom: nextZoom,
            ),
            viewportWidth: viewWidth,
            horizontalScrollController: horizontalScrollController,
            verticalScrollController: verticalScrollController,
            isMounted: () => context.mounted,
            isPinching: () => true,
          );
          return true;
        }

        void handlePointerScroll(PointerScrollEvent event) {
          final keysPressed = HardwareKeyboard.instance.logicalKeysPressed;
          if (handlePointerZoom(event, keysPressed)) {
            return;
          }
          final isShiftPressed =
              keysPressed.contains(LogicalKeyboardKey.shiftLeft) || keysPressed.contains(LogicalKeyboardKey.shiftRight);

          var scrollDx = event.scrollDelta.dx;
          var scrollDy = event.scrollDelta.dy;

          if (allowHorizontalPan && isShiftPressed && scrollDx == 0 && scrollDy != 0) {
            scrollDx = scrollDy;
            scrollDy = 0;
          }

          if (scrollDx == 0 && scrollDy == 0) {
            return;
          }

          gesture.applyRawPanDelta(delta: Offset(-scrollDx, -scrollDy), allowHorizontal: allowHorizontalPan);
        }

        void startLongPress(Offset globalPosition) {
          if (pinch.isPinching) {
            return;
          }
          if (gesture.isCellHandleDragging) {
            return;
          }
          if (gestureState.active) {
            return;
          }
          final viewportPosition = viewportPositionFromGlobal(globalPosition);
          if (viewportPosition == null) {
            return;
          }
          scope.inputController.commitComposing();

          longPressPosition.value = viewportPosition;
          if (!gestureState.startLongPress()) {
            return;
          }

          final draggingHandle = state.state.draggingHandle;
          final anchorHandle = draggingHandle == SelectionHandleType.from
              ? state.state.selection?.toBounds
              : state.state.selection?.fromBounds;

          gesture.beginLongPressSession(
            touchPosition: globalPosition,
            handleScreenPosition: gesture.getHandleStemCenter(fromHandle ?? toHandle, scope.geometry),
            anchorHandle: anchorHandle,
          );
        }

        void updateLongPress(Offset viewportPosition) {
          if (pinch.isPinching) {
            return;
          }
          if (!gestureState.longPressing || gestureState.active) {
            return;
          }

          final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
          longPressPosition.value = viewportPosition;

          if (pageIdx >= 0) {
            final pointerX = gesture.getPointerX(viewportPosition.dx);
            scope.controller.dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'clickCount': 1,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
            scope.controller.dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
            scope.controller.scrollIntoView();
          }

          gesture.handleAutoScroll(
            y: viewportPosition.dy,
            x: viewportPosition.dx,
            viewWidth: viewWidth,
            viewHeight: viewHeight,
            handleDragPosition: handleDragPosition,
            longPressPosition: longPressPosition,
            dropPosition: dropPosition,
          );
        }

        void endLongPress() {
          if (pinch.isPinching) {
            return;
          }
          if (!gestureState.longPressing || gestureState.active) {
            return;
          }
          longPressPosition.value = null;
          gesture.stopAutoScroll();
          gestureState.endLongPress();
        }

        final listView = ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false, dragDevices: isSelecting ? {} : null),
          child: SingleChildScrollView(
            controller: verticalScrollController,
            physics: isSelecting ? const NeverScrollableScrollPhysics() : const _NonGestureBouncingScrollPhysics(),
            child: Builder(
              builder: (_) {
                final content = RawGestureDetector(
                  gestures: {
                    ConditionalLongPressGestureRecognizer:
                        GestureRecognizerFactoryWithHandlers<ConditionalLongPressGestureRecognizer>(
                          () => ConditionalLongPressGestureRecognizer(
                            condition: (globalPosition) {
                              if (gesture.isCellHandleDragging) {
                                return false;
                              }
                              if (gestureState.active) {
                                return false;
                              }
                              final viewportPosition = viewportPositionFromGlobal(globalPosition);
                              if (viewportPosition == null) {
                                return true;
                              }
                              final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
                              final pointerX = gesture.getPointerX(viewportPosition.dx);
                              return scope.editor.isSelectionHit(pageIdx, pointerX, localY);
                            },
                            duration: const Duration(milliseconds: 500),
                          ),
                          (ConditionalLongPressGestureRecognizer instance) {
                            instance
                              ..onLongPressStart = (details) {
                                startLongPress(details.globalPosition);
                              }
                              ..onLongPressEnd = (details) {
                                endLongPress();
                              };
                          },
                        ),
                  },
                  child: Column(
                    children: [
                      _MeasuredTitleFields(scope: scope),
                      _TrackedHorizontalScrollView(
                        controller: horizontalScrollController,
                        physics: horizontalPhysics,
                        child: SizedBox(
                          width: math.max(contentWidth, viewWidth),
                          child: Align(
                            alignment: Alignment.topCenter,
                            child: Container(
                              width: contentWidth,
                              padding: EdgeInsets.only(
                                left: geo.horizontalPadding,
                                right: geo.horizontalPadding,
                                bottom: contentBottomPadding,
                              ),
                              child: Column(
                                children: [
                                  for (var i = 0; i < pages.length; i++) ...[
                                    _PageSlot(
                                      key: ValueKey(i),
                                      pageIndex: i,
                                      pageTop: geo.titleAreaHeight + offsets[i],
                                      pageBottom: geo.titleAreaHeight + offsets[i] + geo.pageHeightAt(i),
                                      activeCursorPageIdx: renderedCursor?.pageIdx,
                                    ),
                                  ],
                                ],
                              ),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                );

                return EditorDraggable(
                  gesture: gesture,
                  resolveDragLocation: (globalPosition) {
                    final viewportPosition = viewportPositionFromGlobal(globalPosition);
                    if (viewportPosition == null) {
                      return null;
                    }
                    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
                    final pointerX = gesture.getPointerX(viewportPosition.dx);
                    return (localPosition: viewportPosition, pageIdx: pageIdx, localY: localY, pointerX: pointerX);
                  },
                  child: content,
                );
              },
            ),
          ),
        );

        void dispatchTap(Offset localPosition) {
          if (pinch.isPinching) {
            return;
          }
          showContextMenu.value = false;

          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);

          if (pageIdx < 0) {
            return;
          }

          scope.inputController.commitComposing();
          scope.inputController.openInput();

          final now = DateTime.now();
          final clickCount = isConsecutiveTap(localPosition: localPosition, now: now) ? 2 : 1;

          gesture.recordTap(now: now, localPosition: localPosition);

          final pointerX = gesture.getPointerX(localPosition.dx);
          final tappedInteractive = scope.editor.isInteractiveHit(pageIdx, pointerX, localY);

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

          final prevCursor = cursor;

          scope.controller.dispatch({
            'type': 'pointerDown',
            'pageIdx': pageIdx,
            'x': pointerX,
            'y': localY,
            'clickCount': clickCount,
            'button': 'primary',
            'modifier': {'shift': isShiftHeader, 'ctrl': false, 'alt': false, 'meta': false},
          });
          scope.controller.dispatch({
            'type': 'pointerUp',
            'pageIdx': pageIdx,
            'x': pointerX,
            'y': localY,
            'button': 'primary',
            'modifier': {'shift': isShiftHeader, 'ctrl': false, 'alt': false, 'meta': false},
          });

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
                  isCollapsed &&
                  newState.cursor != null &&
                  prevCursor != null &&
                  newState.cursor!.isSamePosition(prevCursor);

              if (isSameCursor) {
                if (!tappedInteractive && !wasContextMenuOpen.value) {
                  showContextMenu.value = true;
                }
                return;
              }

              if (tappedInteractive) {
                return;
              }

              scope.controller.scrollIntoView();
            }),
          );
        }

        Widget buildSelectionHandle(SelectionHandleInfo handle, SelectionHandleType type) {
          final scaledHandle = handle.copyWith(height: geo.toDisplayY(handle.height));
          return SelectionHandle(
            handleInfo: scaledHandle,
            type: type,
            onDragDown: onHandleDragDown,
            onDragStart: onHandleDragStart,
            onDragUpdate: onHandleDragUpdate,
            onDragEnd: onHandleDragEnd,
          );
        }

        final gestureDetector = GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: (details) {
            if (pinch.isPinching) {
              return;
            }
            wasContextMenuOpen.value = showContextMenu.value;
            if (showContextMenu.value) {
              showContextMenu.value = false;
            }

            gesture.cancelTapTimer();

            if (isConsecutiveTap(localPosition: details.localPosition, now: DateTime.now())) {
              gesture.setTapDispatched(true);
              if (dispatchDoubleTapSelection(details.localPosition)) {
                prepareDoubleTapDrag(details.localPosition);
              }
              return;
            }

            gesture
              ..setTapDispatched(false)
              ..scheduleTapTimer(const Duration(milliseconds: 150), () {
                final pointerX = gesture.getPointerX(details.localPosition.dx);
                final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);

                final canDrag = scope.editor.isSelectionHit(pageIdx, pointerX, localY);

                if (canDrag) {
                  gesture.setTapDispatched(true);
                  return;
                }

                if (hasRangeSelection) {
                  // Keep current selection until long-press gesture resolves.
                  return;
                }

                gesture.setTapDispatched(true);
                dispatchTap(details.localPosition);
              });
          },
          onTapUp: (details) {
            if (pinch.isPinching) {
              return;
            }
            if (gestureState.dragging) {
              return;
            }
            gestureState.clearPending();
            gesture.cancelTapTimer();
            if (!gesture.tapDispatched) {
              dispatchTap(details.localPosition);
            }
          },
          onTapCancel: () {
            if (pinch.isPinching) {
              return;
            }
            if (gestureState.active) {
              return;
            }
            gestureState.clearPending();
            gesture.cancelTapTimer();
          },

          onPanDown: (details) {
            if (pinch.isPinching) {
              return;
            }
            if (isSelecting) {
              return;
            }
            gesture.holdScrollPositions();
          },
          onPanStart: (details) {
            if (pinch.isPinching) {
              return;
            }
            if (gestureState.active) {
              return;
            }
            gesture.startScrollDrag(details: details, allowHorizontal: allowHorizontalPan);
          },
          onPanUpdate: (details) {
            if (pinch.isPinching) {
              return;
            }
            gesture.updateScrollDrag(details);
          },
          onPanEnd: (details) {
            if (pinch.isPinching) {
              return;
            }
            if (gestureState.active) {
              return;
            }
            gesture.endScrollDrag(details);
          },
          onPanCancel: () {
            if (pinch.isPinching) {
              return;
            }
            if (gestureState.active) {
              return;
            }
            gesture.cancelScrollDrag();
          },
          child: listView,
        );

        return DropRegion(
          formats: Formats.standardFormats,
          hitTestBehavior: HitTestBehavior.translucent,
          onDropOver: (event) {
            if (pinch.isPinching) {
              return DropOperation.none;
            }
            final item = event.session.items.firstOrNull;
            if (item == null) {
              return DropOperation.none;
            }

            final position = event.position.local;
            final (pIdx, localY) = getPageAtPosition(position.dy);

            final pointerX = gesture.getPointerX(position.dx);

            dropPosition.value = position;
            gesture.handleAutoScroll(
              y: position.dy,
              x: position.dx,
              viewWidth: viewWidth,
              viewHeight: viewHeight,
              handleDragPosition: handleDragPosition,
              longPressPosition: longPressPosition,
              dropPosition: dropPosition,
            );

            scope.dndController.handleDragOver(pIdx, pointerX, localY);

            final localData = item.localData;
            if (localData is Map && localData['isInternal'] == true) {
              return DropOperation.move;
            }
            return DropOperation.copy;
          },
          onDropEnter: (event) {
            if (pinch.isPinching) {
              return;
            }
            scope.dndController.handleDragEnter();
          },
          onDropLeave: (event) {
            dropPosition.value = null;
            gesture.stopAutoScroll();
            scope.dndController.handleDragLeave();
          },
          onPerformDrop: (event) async {
            if (pinch.isPinching) {
              return;
            }
            dropPosition.value = null;
            gesture.stopAutoScroll();

            final position = event.position.local;
            final (pageIdx, localY) = getPageAtPosition(position.dy);

            if (pageIdx < 0) {
              scope.dndController.handleDragEnd();
              return;
            }

            final pointerX = gesture.getPointerX(position.dx);

            unawaited(HapticFeedback.lightImpact());
            await scope.dndController.handleDrop(pageIdx: pageIdx, x: pointerX, y: localY, session: event.session);
          },
          child: Listener(
            behavior: HitTestBehavior.translucent,
            onPointerSignal: (event) {
              if (event is! PointerScrollEvent) {
                return;
              }
              if (pinch.isPinching) {
                return;
              }
              handlePointerScroll(event);
            },
            onPointerDown: (event) {
              clearResumedPanState();
              pinch.addPointer(event.pointer, event.localPosition);
              if (pinch.pointerCount >= 2) {
                beginPinchIfNeeded();
              }
            },
            onPointerMove: (event) {
              final previousPointerPosition = pinch.pointerPosition(event.pointer);
              if (pinch.containsPointer(event.pointer)) {
                pinch.updatePointer(event.pointer, event.localPosition);
              }

              if (pinch.isPinching) {
                updatePinchZoom();
                return;
              }

              if (resumedPanPointer.value == event.pointer) {
                final previous = resumedPanLastLocalPosition.value;
                resumedPanLastLocalPosition.value = event.localPosition;

                if (previous == null) {
                  return;
                }

                if (isSelecting || gestureState.active || gesture.hasAnyHandleDrag) {
                  clearResumedPanState();
                  return;
                }

                final delta = event.localPosition - previous;
                if (!resumedPanActive.value) {
                  if (delta.distance < 1) {
                    return;
                  }
                  gesture.startScrollDrag(
                    details: DragStartDetails(globalPosition: event.position, localPosition: previous),
                    allowHorizontal: allowHorizontalPan,
                  );
                  resumedPanActive.value = true;
                }

                if (delta.distance > 0) {
                  gesture.updateScrollDrag(
                    DragUpdateDetails(
                      globalPosition: event.position,
                      localPosition: event.localPosition,
                      delta: delta,
                      sourceTimeStamp: event.timeStamp,
                    ),
                  );
                }
                return;
              }

              if (gestureState.pending) {
                final startPosition = gestureState.start;
                if (startPosition != null && (event.localPosition - startPosition).distance >= 4) {
                  startDoubleTapDrag(startPosition);
                  updateDoubleTapDragSelection(event.localPosition);
                }
                return;
              }
              if (gestureState.dragging) {
                updateDoubleTapDragSelection(event.localPosition);
                return;
              }

              if (gestureState.longPressing) {
                updateLongPress(event.localPosition);
                return;
              }

              final isSinglePointer = pinch.pointerCount == 1;
              final canRawPan =
                  isSinglePointer &&
                  previousPointerPosition != null &&
                  !isSelecting &&
                  !gesture.hasAnyHandleDrag &&
                  !gesture.hasScrollDrag;
              if (canRawPan) {
                final delta = event.localPosition - previousPointerPosition;
                if (delta.distance >= 1) {
                  gesture.applyRawPanDelta(delta: delta, allowHorizontal: allowHorizontalPan);
                }
              }
            },
            onPointerUp: (event) {
              if (resumedPanPointer.value == event.pointer) {
                if (resumedPanActive.value) {
                  gesture.endScrollDrag(DragEndDetails());
                }
                clearResumedPanState();
              }

              final wasPinching = pinch.isPinching;
              pinch.removePointer(event.pointer);
              if (pinch.pointerCount < 2) {
                endPinchIfNeeded();
              }

              if (wasPinching && pinch.pointerCount == 1) {
                final remaining = pinch.singlePointerEntry!;
                resumedPanPointer.value = remaining.key;
                resumedPanLastLocalPosition.value = remaining.value;
                resumedPanActive.value = false;
              }

              if (wasPinching) {
                return;
              }

              if (gestureState.dragging) {
                endDoubleTapDrag();
                return;
              }
              gestureState.clearPending();
              endLongPress();
              endTextHandleDrag();
            },
            onPointerCancel: (event) {
              if (resumedPanPointer.value == event.pointer) {
                if (resumedPanActive.value) {
                  gesture.cancelScrollDrag();
                }
                clearResumedPanState();
              }

              final wasPinching = pinch.isPinching;
              pinch.removePointer(event.pointer);
              if (pinch.pointerCount < 2) {
                endPinchIfNeeded();
              }

              if (wasPinching && pinch.pointerCount == 1) {
                final remaining = pinch.singlePointerEntry!;
                resumedPanPointer.value = remaining.key;
                resumedPanLastLocalPosition.value = remaining.value;
                resumedPanActive.value = false;
              }

              if (wasPinching) {
                return;
              }

              if (gestureState.dragging) {
                endDoubleTapDrag();
                return;
              }
              gestureState.clearPending();
              endLongPress();
              endTextHandleDrag();
            },
            child: Stack(
              clipBehavior: Clip.none,
              children: [
                NotificationListener<ScrollMetricsNotification>(
                  onNotification: (_) {
                    handleMetricsRevision.value++;
                    return false;
                  },
                  child: gestureDetector,
                ),
                TableOverlay(
                  gesture: gesture,
                  viewWidth: viewWidth,
                  viewHeight: viewHeight,
                  dropPosition: dropPosition,
                  globalToViewport: viewportPositionFromGlobal,
                ),
                ListenableBuilder(
                  listenable: Listenable.merge([
                    verticalScrollController,
                    horizontalScrollController,
                    handleMetricsRevision,
                  ]),
                  builder: (context, _) {
                    final fromPos = gesture.getHandlePosition(fromHandle, geo);
                    final toPos = gesture.getHandlePosition(toHandle, geo);

                    return Stack(
                      clipBehavior: Clip.none,
                      children: [
                        if (isFocused && fromHandle != null && fromPos != null)
                          if (!isTableCellSelectorSelection)
                            Positioned(
                              left: fromPos.dx,
                              top: fromPos.dy,
                              child: buildSelectionHandle(fromHandle, SelectionHandleType.from),
                            ),
                        if (isFocused && toHandle != null && toPos != null)
                          if (!isTableCellSelectorSelection)
                            Positioned(
                              left: toPos.dx,
                              top: toPos.dy,
                              child: buildSelectionHandle(toHandle, SelectionHandleType.to),
                            ),
                      ],
                    );
                  },
                ),
                if (showContextMenu.value &&
                    longPressPosition.value == null &&
                    !gesture.hasTextHandleDrag &&
                    !gestureState.active)
                  SelectionContextMenu(clipboard: clipboard, onDismiss: () => showContextMenu.value = false),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _TrackedHorizontalScrollView extends HookWidget {
  const _TrackedHorizontalScrollView({required this.controller, required this.physics, required this.child});

  final ScrollController controller;
  final ScrollPhysics physics;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final trackedPositionRef = useRef<ScrollPosition?>(null);

    void registerPosition(ScrollPosition position) {
      final previous = trackedPositionRef.value;
      if (!identical(previous, position)) {
        if (previous != null) {
          clearPreferredHorizontalScrollPosition(controller, previous);
        }
        trackedPositionRef.value = position;
      }
      setPreferredHorizontalScrollPosition(controller, position);
    }

    useEffect(() {
      return () {
        final tracked = trackedPositionRef.value;
        if (tracked != null) {
          clearPreferredHorizontalScrollPosition(controller, tracked);
        }
        trackedPositionRef.value = null;
      };
    }, [controller]);

    return NotificationListener<ScrollNotification>(
      onNotification: (notification) {
        final context = notification.context;
        if (context != null) {
          registerPosition(Scrollable.of(context).position);
        }
        return false;
      },
      child: SingleChildScrollView(
        controller: controller,
        scrollDirection: Axis.horizontal,
        physics: physics,
        child: HookBuilder(
          builder: (context) {
            final position = Scrollable.of(context).position;
            useEffect(() {
              registerPosition(position);
              return null;
            }, [position, controller]);
            return child;
          },
        ),
      ),
    );
  }
}

class _PageSlot extends HookWidget {
  const _PageSlot({
    required this.pageIndex,
    required this.pageTop,
    required this.pageBottom,
    required this.activeCursorPageIdx,
    super.key,
  });

  final int pageIndex;
  final double pageTop;
  final double pageBottom;
  final int? activeCursorPageIdx;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final verticalScrollController = scope.verticalScrollController;

    bool computeVisibility() {
      if (activeCursorPageIdx == pageIndex) {
        return true;
      }
      final verticalPosition = resolveScrollPosition(verticalScrollController);
      if (verticalPosition == null || !verticalPosition.hasContentDimensions) {
        return true;
      }
      final scrollOffset = verticalPosition.pixels;
      final viewHeight = verticalPosition.viewportDimension;
      const cacheExtent = 200.0;
      final viewTop = scrollOffset - cacheExtent;
      final viewBottom = scrollOffset + viewHeight + cacheExtent;
      return pageBottom >= viewTop && pageTop <= viewBottom;
    }

    final visible = useState(computeVisibility());

    useEffect(() {
      void updateVisibility() {
        final nowVisible = computeVisibility();
        if (nowVisible != visible.value) {
          visible.value = nowVisible;
        }
      }

      verticalScrollController.addListener(updateVisibility);
      updateVisibility();
      return () => verticalScrollController.removeListener(updateVisibility);
    }, [verticalScrollController, pageTop, pageBottom, activeCursorPageIdx]);

    final pageHeight = pageBottom - pageTop;

    if (!visible.value) {
      return SizedBox(height: pageHeight + scope.geometry.gapAfterPage(pageIndex));
    }

    return PageItem(pageIndex: pageIndex);
  }
}

class _NonGestureBouncingScrollPhysics extends BouncingScrollPhysics {
  const _NonGestureBouncingScrollPhysics({super.parent});

  @override
  _NonGestureBouncingScrollPhysics applyTo(ScrollPhysics? ancestor) {
    return _NonGestureBouncingScrollPhysics(parent: buildParent(ancestor));
  }

  @override
  bool shouldAcceptUserOffset(ScrollMetrics position) => false;
}

class _MeasuredTitleFields extends HookWidget {
  const _MeasuredTitleFields({required this.scope});

  final ContentScope scope;

  @override
  Widget build(BuildContext context) {
    void measureHeight() {
      final renderBox = context.findRenderObject() as RenderBox?;
      if (renderBox != null && renderBox.hasSize) {
        final height = renderBox.size.height;
        if (scope.titleAreaHeight.value != height) {
          scope.titleAreaHeight.value = height;
        }
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) => measureHeight());
      return null;
    });

    final title = useValueListenable(scope.title);
    final subtitle = useValueListenable(scope.subtitle);

    return LayoutBuilder(
      builder: (context, constraints) {
        return TitleFields(
          title: title,
          subtitle: subtitle,
          onEnterDocument: () {
            scope.inputController.openInput();
            scope.controller.dispatch({'type': 'navigate', 'direction': 'documentStart', 'extend': false});
            scope.controller.scrollIntoView();
          },
          pageWidth: constraints.maxWidth,
          onFieldTap: scope.inputController.clearFocus,
        );
      },
    );
  }
}
