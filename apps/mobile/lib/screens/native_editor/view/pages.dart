import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/context_menu.dart';
import 'package:typie/screens/native_editor/view/editor_draggable.dart';
import 'package:typie/screens/native_editor/view/interaction/controller.dart';
import 'package:typie/screens/native_editor/view/interaction/state.dart';
import 'package:typie/screens/native_editor/view/page.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/selection.dart';
import 'package:typie/screens/native_editor/view/table_overlay.dart';
import 'package:typie/screens/native_editor/view/title.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';
import 'package:typie/services/preference.dart';

class PageList extends HookWidget {
  const PageList({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final pref = useService<Pref>();
    final state = useListenable(scope.controller);
    final sheetBottomInset = useValueListenable(scope.controller.sheetBottomInset);
    final viewportTopInset = useValueListenable(scope.viewportTopInset);

    final pages = state.state.pages;
    final cursor = state.state.cursor;
    final presentedViewport = useValueListenable(scope.presentedViewport);
    final renderedCursor = presentedViewport.cursor;
    final isFocused = state.state.isFocused;
    final selection = state.state.selection;
    final fromHandle = state.state.selection?.fromBounds;
    final toHandle = state.state.selection?.toBounds;
    final dropIndicator = state.state.dropIndicator;

    final interactionSnapshot = useValueListenable(scope.interactionSnapshot);
    final isSelecting = interactionSnapshot.isSelecting;
    final isDropping = interactionSnapshot.mode == InteractionMode.dndExternal;
    final isDndActive = interactionSnapshot.isDndActive;

    final tableOverlays = useValueListenable(scope.controller.tableOverlays);
    final isTableCellSelectorSelection = tableOverlays.any((overlay) => overlay.isFocused && overlay.showCellSelector);

    final verticalScrollController = scope.verticalScrollController;
    final horizontalScrollController = scope.horizontalScrollController;
    final handleMetricsRevision = useValueNotifier(0);

    useValueListenable(scope.titleAreaHeight);
    useValueListenable(scope.displayZoom);

    final showContextMenu = useState(false);
    final wasContextMenuOpen = useRef(false);
    final clipboard = useMemoized(EditorClipboard.new);
    final viewportWidth = useValueNotifier<double>(0);
    final viewportSize = useValueNotifier(Size.zero);

    final longPressPosition = scope.longPressPosition;
    final interactionRuntime = useEditorInteractionRuntime(
      context: context,
      scope: scope,
      showContextMenu: showContextMenu,
      wasContextMenuOpen: wasContextMenuOpen,
      viewportSize: viewportSize,
    );
    final interactionController = interactionRuntime.controller;
    final interactionRegionKey = interactionRuntime.interactionRegionKey;
    final dropPosition = interactionRuntime.dropPosition;

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
    final syncedFromHandle = useRef<SelectionHandleInfo?>(fromHandle);
    final syncedToHandle = useRef<SelectionHandleInfo?>(toHandle);
    final prevSelectionRangeKey = useRef<String?>(null);
    final wasSelecting = useRef(false);
    final previousDropIndicatorKey = useRef<String?>(null);
    final isFrameSynchronized = identical(presentedViewport.renderVersion, state.state.renderVersion);
    final shouldUseLiveHandlePosition = isFrameSynchronized;

    useEffect(() {
      if (shouldUseLiveHandlePosition) {
        syncedFromHandle.value = fromHandle;
        syncedToHandle.value = toHandle;
      }
      return null;
    }, [shouldUseLiveHandlePosition, fromHandle, toHandle]);

    final renderedFromHandle = shouldUseLiveHandlePosition ? fromHandle : syncedFromHandle.value;
    final renderedToHandle = shouldUseLiveHandlePosition ? toHandle : syncedToHandle.value;

    useEffect(
      () {
        final isLongPressing = interactionSnapshot.isLongPressing;
        final previousSelectionRange = prevSelectionRangeKey.value;
        final currentSelectionRange = selectionRangeKey(selection);
        final isCollapsed = fromHandle == null || toHandle == null;
        final handleDragCanceledByTableSelection =
            isTableCellSelectorSelection &&
            !interactionController.isTableCellHandleDragging &&
            interactionController.hasSelectionHandleDrag;
        final justFinishedSelecting = wasSelecting.value && !isSelecting;
        final selectionChanged = fromHandle != prevFromHandle.value || toHandle != prevToHandle.value;
        final hasPreviousSelectionSnapshot = previousSelectionRange != null;

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
            previousSelectionRange != null &&
            currentSelectionRange != null &&
            previousSelectionRange != currentSelectionRange &&
            !handlesJustAppeared;

        final shouldResetSelectionHandleDrag =
            !interactionController.isTableCellHandleDragging && (isCollapsed || handleDragCanceledByTableSelection);

        if (handlesJustAppeared) {
          unawaited(HapticFeedback.selectionClick());
        }

        if (!interactionController.isTableCellHandleDragging &&
            interactionController.hasSelectionHandleDrag &&
            anyHandleMoved) {
          unawaited(HapticFeedback.selectionClick());
        }

        if (longPressSelectionMoved) {
          unawaited(HapticFeedback.selectionClick());
        }

        if (shouldResetSelectionHandleDrag) {
          interactionController.stopSelectionHandlesAndAutoScroll();
          showContextMenu.value = false;
        } else if (!isSelecting && !interactionController.isDoubleTapDragActive) {
          if (justFinishedSelecting || selectionChanged) {
            showContextMenu.value = true;
          }
        }

        if (isSelecting || interactionController.isDoubleTapDragActive) {
          showContextMenu.value = false;
        }

        wasSelecting.value = isSelecting;
        prevFromHandle.value = fromHandle;
        prevToHandle.value = toHandle;
        prevSelectionRangeKey.value = currentSelectionRange;
        return null;
      },
      [selection, fromHandle, toHandle, isSelecting, isTableCellSelectorSelection, interactionSnapshot.isLongPressing],
    );

    useEffect(() {
      final nextKey = dropIndicatorKey(dropIndicator);
      final previousKey = previousDropIndicatorKey.value;
      if (isDropping && previousKey != null && nextKey != null && previousKey != nextKey) {
        unawaited(HapticFeedback.selectionClick());
      }
      previousDropIndicatorKey.value = nextKey;
      return null;
    }, [dropIndicator, isDropping]);

    useEffect(() {
      if (!isFocused) {
        interactionController.clearTapHistory();
        showContextMenu.value = false;
      }
      return null;
    }, [isFocused, interactionController]);

    useEffect(() {
      if (!interactionSnapshot.isAuxiliaryGesture) {
        return null;
      }
      interactionController.stopInteractionAutoScroll();
      if (showContextMenu.value) {
        showContextMenu.value = false;
      }
      return null;
    }, [interactionSnapshot.isAuxiliaryGesture, interactionController]);

    useEffect(() {
      if (isDndActive) {
        interactionController.cancelInteractionScrollDrag();
      }
      return null;
    }, [isDndActive, interactionController]);

    return EditorInteractionControllerScope(
      controller: interactionController,
      child: LayoutBuilder(
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

          final nextSize = Size(viewWidth, viewHeight);
          if (viewportSize.value != nextSize) {
            viewportSize.value = nextSize;
          }

          final allowHorizontalPan = geo.isPaginated;
          final offsets = geo.computeCumulativePageOffsets();
          final contentWidth = geo.contentWidth;

          final scrollLocked = isSelecting || isDndActive || interactionSnapshot.isAuxiliaryGesture;
          final horizontalPhysics = scrollLocked || !allowHorizontalPan
              ? const NeverScrollableScrollPhysics()
              : const NonGestureBouncingScrollPhysics();
          final effectiveViewportHeight = math.max<double>(0, viewHeight - sheetBottomInset);

          final contentBottomPadding = geo.bottomPadding(
            viewportHeight: effectiveViewportHeight,
            cursor: cursor,
            typewriterEnabled: pref.typewriterEnabled,
            typewriterPosition: pref.typewriterPosition,
            viewportTopInset: viewportTopInset,
          );

          final listView = Padding(
            padding: EdgeInsets.only(bottom: sheetBottomInset),
            child: ScrollConfiguration(
              behavior: ScrollConfiguration.of(
                context,
              ).copyWith(scrollbars: false, dragDevices: scrollLocked ? {} : null),
              child: SingleChildScrollView(
                controller: verticalScrollController,
                physics: scrollLocked ? const NeverScrollableScrollPhysics() : const NonGestureBouncingScrollPhysics(),
                child: Builder(
                  builder: (_) {
                    final content = RawGestureDetector(
                      gestures: {
                        ConditionalLongPressGestureRecognizer:
                            GestureRecognizerFactoryWithHandlers<ConditionalLongPressGestureRecognizer>(
                              () => ConditionalLongPressGestureRecognizer(
                                condition: interactionController.shouldRejectLongPress,
                                duration: const Duration(milliseconds: 500),
                              ),
                              (ConditionalLongPressGestureRecognizer instance) {
                                instance
                                  ..onLongPressStart = (details) {
                                    interactionController.startLongPress(details.globalPosition);
                                  }
                                  ..onLongPressEnd = (details) {
                                    interactionController.endLongPress();
                                  };
                              },
                            ),
                      },
                      child: TrackedHorizontalScrollView(
                        controller: horizontalScrollController,
                        physics: horizontalPhysics,
                        child: SizedBox(
                          width: math.max(contentWidth, viewWidth),
                          child: Align(
                            alignment: Alignment.topCenter,
                            child: SizedBox(
                              width: contentWidth,
                              child: Column(
                                children: [
                                  MeasuredTitleFields(scope: scope, pageWidth: contentWidth),
                                  Padding(
                                    padding: EdgeInsets.only(
                                      left: geo.horizontalPadding,
                                      right: geo.horizontalPadding,
                                      bottom: contentBottomPadding,
                                    ),
                                    child: Column(
                                      children: [
                                        for (var i = 0; i < pages.length; i++) ...[
                                          PageSlot(
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
                                ],
                              ),
                            ),
                          ),
                        ),
                      ),
                    );

                    return EditorDraggable(interactionController: interactionController, child: content);
                  },
                ),
              ),
            ),
          );

          Widget buildSelectionHandle(SelectionHandleInfo handle, SelectionHandleType type) {
            final scaledHandle = handle.copyWith(height: geo.toDisplayY(handle.height));
            return SelectionHandle(
              handleInfo: scaledHandle,
              type: type,
              onDragDown: interactionController.onHandleDragDown,
              onDragStart: interactionController.onHandleDragStart,
              onDragUpdate: interactionController.onHandleDragUpdate,
              onDragEnd: interactionController.onHandleDragEnd,
            );
          }

          final gestureDetector = GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapDown: interactionController.onTapDown,
            onTapUp: interactionController.onTapUp,
            onTapCancel: interactionController.onTapCancel,
            onPanStart: interactionController.onPanStart,
            onPanUpdate: interactionController.onPanUpdate,
            onPanEnd: interactionController.onPanEnd,
            onPanCancel: interactionController.onPanCancel,
            child: listView,
          );

          return DropRegion(
            formats: Formats.standardFormats,
            hitTestBehavior: HitTestBehavior.translucent,
            onDropOver: interactionController.onDropOver,
            onDropEnter: interactionController.onDropEnter,
            onDropLeave: interactionController.onDropLeave,
            onDropEnded: interactionController.onDropEnded,
            onPerformDrop: interactionController.onPerformDrop,
            child: Listener(
              key: interactionRegionKey,
              behavior: HitTestBehavior.translucent,
              onPointerSignal: interactionController.onPointerSignal,
              onPointerDown: interactionController.onPointerDown,
              onPointerMove: interactionController.onPointerMove,
              onPointerUp: interactionController.onPointerUp,
              onPointerCancel: interactionController.onPointerCancel,
              child: Stack(
                fit: StackFit.expand,
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
                    interactionController: interactionController,
                    viewWidth: viewWidth,
                    viewHeight: viewHeight,
                    dropPosition: dropPosition,
                  ),
                  ListenableBuilder(
                    listenable: Listenable.merge([
                      verticalScrollController,
                      horizontalScrollController,
                      handleMetricsRevision,
                    ]),
                    builder: (context, _) {
                      final fromPos = interactionController.selectionHandleViewportPosition(renderedFromHandle, geo);
                      final toPos = interactionController.selectionHandleViewportPosition(renderedToHandle, geo);

                      return Stack(
                        clipBehavior: Clip.none,
                        children: [
                          if (isFocused && renderedFromHandle != null && fromPos != null)
                            if (!isTableCellSelectorSelection)
                              Positioned(
                                left: fromPos.dx,
                                top: fromPos.dy,
                                child: buildSelectionHandle(renderedFromHandle, SelectionHandleType.from),
                              ),
                          if (isFocused && renderedToHandle != null && toPos != null)
                            if (!isTableCellSelectorSelection)
                              Positioned(
                                left: toPos.dx,
                                top: toPos.dy,
                                child: buildSelectionHandle(renderedToHandle, SelectionHandleType.to),
                              ),
                        ],
                      );
                    },
                  ),
                  if (showContextMenu.value &&
                      longPressPosition.value == null &&
                      !interactionController.hasSelectionHandleDrag &&
                      !interactionController.isDoubleTapDragActive)
                    SelectionContextMenu(clipboard: clipboard, onDismiss: () => showContextMenu.value = false),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}

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

String? dropIndicatorKey(DropIndicatorInfo? info) {
  if (info == null) {
    return null;
  }
  return '${info.pageIdx}:${info.x}:${info.y}:${info.width}:${info.height}';
}

class TrackedHorizontalScrollView extends HookWidget {
  const TrackedHorizontalScrollView({required this.controller, required this.physics, required this.child, super.key});

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

class PageSlot extends HookWidget {
  const PageSlot({
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

class NonGestureBouncingScrollPhysics extends BouncingScrollPhysics {
  const NonGestureBouncingScrollPhysics({super.parent});

  @override
  NonGestureBouncingScrollPhysics applyTo(ScrollPhysics? ancestor) {
    return NonGestureBouncingScrollPhysics(parent: buildParent(ancestor));
  }

  @override
  bool shouldAcceptUserOffset(ScrollMetrics position) => false;
}

class OverscrollSafeScrollController extends ScrollController {
  @override
  ScrollPosition createScrollPosition(ScrollPhysics physics, ScrollContext context, ScrollPosition? oldPosition) {
    return _OverscrollSafeScrollPosition(physics: physics, context: context, oldPosition: oldPosition);
  }
}

class _OverscrollSafeScrollPosition extends ScrollPositionWithSingleContext {
  _OverscrollSafeScrollPosition({required super.physics, required super.context, super.oldPosition});

  @override
  void correctBy(double correction) {
    if (correction != 0.0 && activity is BallisticScrollActivity) {
      return;
    }
    super.correctBy(correction);
  }
}

class MeasuredTitleFields extends HookWidget {
  const MeasuredTitleFields({required this.scope, required this.pageWidth, super.key});

  final ContentScope scope;
  final double pageWidth;

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

    return TitleFields(
      title: title,
      subtitle: subtitle,
      onEnterDocument: () {
        scope.inputController.openInput();
        scope.controller.dispatch({'type': 'navigate', 'direction': 'documentStart', 'extend': false});
        scope.controller.scrollIntoView();
      },
      pageWidth: pageWidth,
      onFieldTap: scope.inputController.clearFocus,
    );
  }
}
