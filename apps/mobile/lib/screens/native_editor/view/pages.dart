import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/controller/clipboard.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/context_menu.dart';
import 'package:typie/screens/native_editor/view/page.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/selection.dart';
import 'package:typie/services/preference.dart';

class PageList extends HookWidget {
  const PageList({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final pref = useService<Pref>();
    final state = useListenable(scope.controller);

    final layout = state.state.layout!;
    final cursor = state.state.cursor;
    final isFocused = state.state.isFocused;
    final isSelecting = state.state.isSelecting;
    final fromHandle = state.state.fromHandle;
    final toHandle = state.state.toHandle;

    final verticalScrollController = scope.verticalScrollController;
    final horizontalScrollController = scope.horizontalScrollController;
    final editor = scope.editor;

    useValueListenable(scope.titleAreaHeight);

    (int pageIdx, double localY) getPageAtPosition(double y) {
      final geo = scope.geometry;
      final offsets = geo.computeCumulativePageOffsets();
      final absoluteY = y + verticalScrollController.offset;

      if (absoluteY < geo.titleAreaHeight) {
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

      final pageIdx = (low - 1).clamp(0, geo.layout.pageCount - 1);
      final localY = adjustedY - offsets[pageIdx];
      return (pageIdx, localY);
    }

    final showContextMenu = useState(false);
    final clipboard = useMemoized(EditorClipboard.new);
    final pendingContextMenu = useRef(false);

    final longPressPosition = scope.longPressPosition;
    final handleDragPosition = scope.handleDragPosition;
    final draggingHandleType = useState<SelectionHandleType?>(null);
    final pointerDownTouchPosition = useRef<Offset?>(null);
    final dragStartTouchPosition = useRef<Offset?>(null);
    final dragStartHandleScreenPosition = useRef<Offset?>(null);
    final dragAnchorHandle = useRef<SelectionHandleInfo?>(null);
    final lastTapTime = useRef<DateTime?>(null);
    final lastTapPosition = useRef<Offset?>(null);
    final tapTimer = useRef<Timer?>(null);
    final tapDispatched = useRef(false);
    final autoScrollTimer = useRef<Timer?>(null);

    const edgeThreshold = 60.0;
    const minScrollSpeed = 4.0;
    const maxScrollSpeed = 16.0;
    final verticalEdgeDistance = useRef<double>(0);
    final horizontalEdgeDistance = useRef<double>(0);
    final verticalDirection = useRef<double>(0);
    final horizontalDirection = useRef<double>(0);
    final autoScrollViewSize = useRef<Size>(Size.zero);
    final lastDispatchedPosition = useRef<(int, double, double)?>(null);
    final verticalDrag = useRef<Drag?>(null);
    final horizontalDrag = useRef<Drag?>(null);

    void stopAutoScroll() {
      autoScrollTimer.value?.cancel();
      autoScrollTimer.value = null;
      verticalDirection.value = 0;
      horizontalDirection.value = 0;
      lastDispatchedPosition.value = null;
    }

    void startAutoScroll() {
      if (autoScrollTimer.value != null) {
        return;
      }
      autoScrollTimer.value = Timer.periodic(const Duration(milliseconds: 16), (_) {
        final viewHeight = autoScrollViewSize.value.height;
        final viewWidth = autoScrollViewSize.value.width;
        final activePosition = handleDragPosition.value ?? longPressPosition.value;
        var scrolledY = activePosition?.dy ?? 0;
        var scrolledX = activePosition?.dx ?? 0;

        if (verticalDirection.value != 0) {
          final proximity = 1.0 - (verticalEdgeDistance.value / edgeThreshold).clamp(0.0, 1.0);
          final scrollSpeed = minScrollSpeed + proximity * (maxScrollSpeed - minScrollSpeed);

          final currentOffset = verticalScrollController.offset;
          final newOffset = (currentOffset + verticalDirection.value * scrollSpeed).clamp(
            0.0,
            verticalScrollController.position.maxScrollExtent,
          );

          if (newOffset != currentOffset) {
            verticalScrollController.jumpTo(newOffset);
            scrolledY = verticalDirection.value > 0
                ? viewHeight -
                      edgeThreshold +
                      (newOffset >= verticalScrollController.position.maxScrollExtent ? edgeThreshold : 0)
                : newOffset.clamp(0.0, edgeThreshold);
          }
        }

        if (horizontalDirection.value != 0 && horizontalScrollController.hasClients) {
          final proximity = 1.0 - (horizontalEdgeDistance.value / edgeThreshold).clamp(0.0, 1.0);
          final scrollSpeed = minScrollSpeed + proximity * (maxScrollSpeed - minScrollSpeed);

          final currentOffset = horizontalScrollController.offset;
          final newOffset = (currentOffset + horizontalDirection.value * scrollSpeed).clamp(
            0.0,
            horizontalScrollController.position.maxScrollExtent,
          );

          if (newOffset != currentOffset) {
            horizontalScrollController.jumpTo(newOffset);
            scrolledX = horizontalDirection.value > 0
                ? viewWidth -
                      edgeThreshold +
                      (newOffset >= horizontalScrollController.position.maxScrollExtent ? edgeThreshold : 0)
                : newOffset.clamp(0.0, edgeThreshold);
          }
        }

        if (verticalDirection.value == 0 && horizontalDirection.value == 0) {
          stopAutoScroll();
          return;
        }

        if (activePosition != null) {
          final (pageIdx, localY) = getPageAtPosition(scrolledY);

          if (pageIdx < 0) {
            return;
          }

          final pointerX = scrolledX + horizontalScrollController.offset - scope.geometry.horizontalPadding;

          final currentPosition = (pageIdx, pointerX, localY);
          if (lastDispatchedPosition.value == currentPosition) {
            return;
          }
          lastDispatchedPosition.value = currentPosition;

          final handleType = draggingHandleType.value;
          final anchorHandle = dragAnchorHandle.value;
          if (handleType != null && anchorHandle != null) {
            editor.dispatch({
              'type': 'extendSelectionTo',
              'anchorPageIdx': anchorHandle.pageIdx,
              'anchorX': anchorHandle.x,
              'anchorY': anchorHandle.y + anchorHandle.height / 2,
              'headPageIdx': pageIdx,
              'headX': pointerX,
              'headY': localY,
            });
          } else if (handleType == null) {
            editor
              ..dispatch({
                'type': 'pointerDown',
                'pageIdx': pageIdx,
                'x': pointerX,
                'y': localY,
                'clickCount': 1,
                'button': 'primary',
                'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
              })
              ..dispatch({
                'type': 'pointerUp',
                'pageIdx': pageIdx,
                'x': pointerX,
                'y': localY,
                'button': 'primary',
                'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
              });
          }
        }
      });
    }

    useEffect(() => stopAutoScroll, const []);

    useEffect(() {
      return () {
        tapTimer.value?.cancel();
        tapTimer.value = null;
      };
    }, const []);

    useEffect(() {
      return () {
        verticalDrag.value?.cancel();
        horizontalDrag.value?.cancel();
      };
    }, const []);

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

    useEffect(() {
      if (fromHandle == null && toHandle == null) {
        handleDragPosition.value = null;
        draggingHandleType.value = null;
        dragAnchorHandle.value = null;
        showContextMenu.value = false;
        stopAutoScroll();
      } else if (pendingContextMenu.value) {
        pendingContextMenu.value = false;
        showContextMenu.value = true;
      }
      return null;
    }, [fromHandle, toHandle]);

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewWidth = constraints.maxWidth;
        final viewHeight = constraints.maxHeight;

        double getPointerX(double localX) {
          return localX + horizontalScrollController.offset - scope.geometry.horizontalPadding;
        }

        void handleAutoScroll(double y, double x) {
          autoScrollViewSize.value = Size(viewWidth, viewHeight);

          if (y < edgeThreshold) {
            verticalEdgeDistance.value = y;
            verticalDirection.value = -1;
          } else if (y > viewHeight - edgeThreshold) {
            verticalEdgeDistance.value = viewHeight - y;
            verticalDirection.value = 1;
          } else {
            verticalDirection.value = 0;
          }

          if (x < edgeThreshold) {
            horizontalEdgeDistance.value = x;
            horizontalDirection.value = -1;
          } else if (x > viewWidth - edgeThreshold) {
            horizontalEdgeDistance.value = viewWidth - x;
            horizontalDirection.value = 1;
          } else {
            horizontalDirection.value = 0;
          }

          if (verticalDirection.value != 0 || horizontalDirection.value != 0) {
            startAutoScroll();
          } else {
            stopAutoScroll();
          }
        }

        Offset? getHandlePosition(SelectionHandleInfo? handle) {
          if (handle == null) {
            return null;
          }
          final geo = scope.geometry;
          final offsets = geo.computeCumulativePageOffsets();
          final scrollOffset = verticalScrollController.hasClients ? verticalScrollController.offset : 0.0;
          final hScrollOffset = horizontalScrollController.hasClients ? horizontalScrollController.offset : 0.0;
          final pageTopOffset = geo.titleAreaHeight + offsets[handle.pageIdx];
          final y = pageTopOffset + handle.y - scrollOffset;
          final x = geo.horizontalPadding + handle.x - hScrollOffset;
          return Offset(x, y);
        }

        Offset? getHandleStemCenter(SelectionHandleInfo? handle) {
          final pos = getHandlePosition(handle);
          if (pos == null || handle == null) {
            return null;
          }
          return Offset(pos.dx, pos.dy + handle.height / 2);
        }

        void onHandleDragDown(SelectionHandleType type, DragDownDetails details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          pointerDownTouchPosition.value = renderBox.globalToLocal(details.globalPosition);
        }

        void onHandleDragStart(SelectionHandleType type, DragStartDetails details) {
          draggingHandleType.value = type;
          showContextMenu.value = false;
          scope.controller.setSelecting(true);

          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = pointerDownTouchPosition.value ?? renderBox.globalToLocal(details.globalPosition);
          dragStartTouchPosition.value = touchPosition;

          final handle = type == SelectionHandleType.from ? fromHandle : toHandle;
          dragStartHandleScreenPosition.value = getHandleStemCenter(handle) ?? touchPosition;

          dragAnchorHandle.value = type == SelectionHandleType.from ? toHandle : fromHandle;
        }

        void onHandleDragUpdate(SelectionHandleType type, DragUpdateDetails details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox == null) {
            return;
          }
          final touchPosition = renderBox.globalToLocal(details.globalPosition);
          final startTouch = dragStartTouchPosition.value;
          final startHandleScreen = dragStartHandleScreenPosition.value;
          final anchorHandle = dragAnchorHandle.value;
          if (startTouch == null || startHandleScreen == null || anchorHandle == null) {
            return;
          }

          final delta = touchPosition - startTouch;
          final selectionScreenPosition = startHandleScreen + delta;

          handleDragPosition.value = selectionScreenPosition;

          final (pageIdx, localY) = getPageAtPosition(selectionScreenPosition.dy);
          if (pageIdx >= 0) {
            final pointerX = getPointerX(selectionScreenPosition.dx);
            editor.dispatch({
              'type': 'extendSelectionTo',
              'anchorPageIdx': anchorHandle.pageIdx,
              'anchorX': anchorHandle.x,
              'anchorY': anchorHandle.y + anchorHandle.height / 2,
              'headPageIdx': pageIdx,
              'headX': pointerX,
              'headY': localY,
            });
          }

          handleAutoScroll(touchPosition.dy, touchPosition.dx);
        }

        void onHandleDragEnd(SelectionHandleType type, DragEndDetails details) {
          draggingHandleType.value = null;
          handleDragPosition.value = null;
          dragAnchorHandle.value = null;
          stopAutoScroll();
          scope.controller.setSelecting(false);
          showContextMenu.value = true;
        }

        final geo = scope.geometry;
        final offsets = geo.computeCumulativePageOffsets();
        final contentWidth = layout.pageWidth + geo.horizontalPadding * 2;
        final needsHorizontalScroll = contentWidth > viewWidth;
        final horizontalPhysics = isSelecting || !needsHorizontalScroll
            ? const NeverScrollableScrollPhysics()
            : const _NonGestureBouncingScrollPhysics();

        final contentBottomPadding = geo.bottomPadding(
          viewportHeight: verticalScrollController.hasClients
              ? verticalScrollController.position.viewportDimension
              : viewHeight,
          cursor: cursor,
          typewriterEnabled: pref.typewriterEnabled,
          typewriterPosition: pref.typewriterPosition,
        );

        final listView = ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: SingleChildScrollView(
            controller: horizontalScrollController,
            scrollDirection: Axis.horizontal,
            physics: horizontalPhysics,
            child: SizedBox(
              width: contentWidth,
              child: SingleChildScrollView(
                controller: verticalScrollController,
                physics: isSelecting ? const NeverScrollableScrollPhysics() : const _NonGestureBouncingScrollPhysics(),
                padding: EdgeInsets.only(
                  left: geo.horizontalPadding,
                  right: geo.horizontalPadding,
                  bottom: contentBottomPadding,
                ),
                child: Column(
                  children: [
                    SizedBox(height: geo.titleAreaHeight),
                    for (var i = 0; i < layout.pageCount; i++)
                      _PageSlot(
                        key: ValueKey(i),
                        pageIndex: i,
                        pageTop: geo.titleAreaHeight + offsets[i],
                        pageBottom: geo.titleAreaHeight + offsets[i + 1],
                      ),
                  ],
                ),
              ),
            ),
          ),
        );

        void dispatchTap(Offset localPosition) {
          showContextMenu.value = false;

          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);

          if (pageIdx < 0) {
            return;
          }

          scope.inputController.commitComposing();
          scope.inputController.openInput();

          final now = DateTime.now();
          final prevTime = lastTapTime.value;
          final prevPosition = lastTapPosition.value;

          var clickCount = 1;
          if (prevTime != null && prevPosition != null) {
            final timeDiff = now.difference(prevTime).inMilliseconds;
            final distance = (localPosition - prevPosition).distance;
            if (timeDiff < 300 && distance < 20) {
              clickCount = 2;
            }
          }

          if (clickCount == 2) {
            pendingContextMenu.value = true;
          }

          lastTapTime.value = now;
          lastTapPosition.value = localPosition;

          final prevCursor = cursor;

          final pointerX = getPointerX(localPosition.dx);
          editor
            ..dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'clickCount': clickCount,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            })
            ..dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });

          // 터치 후 커서 위치가 그대로면 컨텍스트 메뉴 표시
          if (clickCount == 1 && prevCursor != null) {
            WidgetsBinding.instance.addPostFrameCallback((_) {
              final newState = scope.controller.state;
              if (newState.fromHandle == null &&
                  newState.toHandle == null &&
                  newState.cursor != null &&
                  prevCursor.pageIdx == newState.cursor!.pageIdx &&
                  prevCursor.x == newState.cursor!.x &&
                  prevCursor.y == newState.cursor!.y) {
                showContextMenu.value = true;
              }
            });
          }
        }

        Widget buildSelectionHandle(SelectionHandleInfo handle, SelectionHandleType type) {
          return SelectionHandle(
            handleInfo: handle,
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
            tapDispatched.value = false;
            tapTimer.value?.cancel();
            tapTimer.value = Timer(const Duration(milliseconds: 150), () {
              tapDispatched.value = true;
              dispatchTap(details.localPosition);
            });
          },
          onTapUp: (details) {
            tapTimer.value?.cancel();
            tapTimer.value = null;
            if (!tapDispatched.value) {
              dispatchTap(details.localPosition);
            }
          },
          onTapCancel: () {
            tapTimer.value?.cancel();
            tapTimer.value = null;
          },
          onLongPressStart: (details) {
            scope.inputController.commitComposing();
            longPressPosition.value = details.localPosition;
            scope.isLongPressing.value = true;
          },
          onLongPressMoveUpdate: (details) {
            final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);
            longPressPosition.value = details.localPosition;

            if (pageIdx >= 0) {
              final pointerX = getPointerX(details.localPosition.dx);
              editor
                ..dispatch({
                  'type': 'pointerDown',
                  'pageIdx': pageIdx,
                  'x': pointerX,
                  'y': localY,
                  'clickCount': 1,
                  'button': 'primary',
                  'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
                })
                ..dispatch({
                  'type': 'pointerUp',
                  'pageIdx': pageIdx,
                  'x': pointerX,
                  'y': localY,
                  'button': 'primary',
                  'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
                });
            }

            handleAutoScroll(details.localPosition.dy, details.localPosition.dx);
          },
          onLongPressEnd: (details) {
            longPressPosition.value = null;
            stopAutoScroll();
            scope.isLongPressing.value = false;
            if (fromHandle != null && toHandle != null) {
              showContextMenu.value = true;
            }
          },
          onPanDown: (details) {
            if (isSelecting) {
              return;
            }
            if (verticalScrollController.hasClients) {
              verticalScrollController.position.hold(() {});
            }
            if (horizontalScrollController.hasClients) {
              horizontalScrollController.position.hold(() {});
            }
          },
          onPanStart: (details) {
            if (isSelecting) {
              return;
            }
            if (verticalScrollController.hasClients) {
              verticalDrag.value = verticalScrollController.position.drag(details, () {
                verticalDrag.value = null;
              });
            }
            if (needsHorizontalScroll && horizontalScrollController.hasClients) {
              horizontalDrag.value = horizontalScrollController.position.drag(details, () {
                horizontalDrag.value = null;
              });
            }
          },
          onPanUpdate: (details) {
            verticalDrag.value?.update(
              DragUpdateDetails(
                globalPosition: details.globalPosition,
                delta: Offset(0, details.delta.dy),
                primaryDelta: details.delta.dy,
                sourceTimeStamp: details.sourceTimeStamp,
              ),
            );
            horizontalDrag.value?.update(
              DragUpdateDetails(
                globalPosition: details.globalPosition,
                delta: Offset(details.delta.dx, 0),
                primaryDelta: details.delta.dx,
                sourceTimeStamp: details.sourceTimeStamp,
              ),
            );
          },
          onPanEnd: (details) {
            verticalDrag.value?.end(
              DragEndDetails(
                velocity: Velocity(pixelsPerSecond: Offset(0, details.velocity.pixelsPerSecond.dy)),
                primaryVelocity: details.velocity.pixelsPerSecond.dy,
              ),
            );
            horizontalDrag.value?.end(
              DragEndDetails(
                velocity: Velocity(pixelsPerSecond: Offset(details.velocity.pixelsPerSecond.dx, 0)),
                primaryVelocity: details.velocity.pixelsPerSecond.dx,
              ),
            );
            verticalDrag.value = null;
            horizontalDrag.value = null;
          },
          onPanCancel: () {
            verticalDrag.value?.cancel();
            horizontalDrag.value?.cancel();
            verticalDrag.value = null;
            horizontalDrag.value = null;
          },
          child: listView,
        );

        return Stack(
          clipBehavior: Clip.none,
          children: [
            gestureDetector,
            ListenableBuilder(
              listenable: Listenable.merge([verticalScrollController, horizontalScrollController]),
              builder: (context, _) {
                final fromPos = getHandlePosition(fromHandle);
                final toPos = getHandlePosition(toHandle);

                return Stack(
                  clipBehavior: Clip.none,
                  children: [
                    if (isFocused && fromHandle != null && fromPos != null)
                      Positioned(
                        left: fromPos.dx,
                        top: fromPos.dy,
                        child: buildSelectionHandle(fromHandle, SelectionHandleType.from),
                      ),
                    if (isFocused && toHandle != null && toPos != null)
                      Positioned(
                        left: toPos.dx,
                        top: toPos.dy,
                        child: buildSelectionHandle(toHandle, SelectionHandleType.to),
                      ),
                  ],
                );
              },
            ),
            if (showContextMenu.value && longPressPosition.value == null && handleDragPosition.value == null)
              SelectionContextMenu(
                clipboard: clipboard,
                onDismiss: () => showContextMenu.value = false,
                onBeforeSelectAll: () => pendingContextMenu.value = true,
              ),
          ],
        );
      },
    );
  }
}

class _PageSlot extends HookWidget {
  const _PageSlot({required this.pageIndex, required this.pageTop, required this.pageBottom, super.key});

  final int pageIndex;
  final double pageTop;
  final double pageBottom;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final verticalScrollController = scope.verticalScrollController;

    bool computeVisibility() {
      if (!verticalScrollController.hasClients) {
        return true;
      }
      final scrollOffset = verticalScrollController.offset;
      final viewHeight = verticalScrollController.position.hasContentDimensions
          ? verticalScrollController.position.viewportDimension
          : 0.0;
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
    }, [verticalScrollController, pageTop, pageBottom]);

    final slotHeight = pageBottom - pageTop;

    if (!visible.value) {
      return SizedBox(height: slotHeight);
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
