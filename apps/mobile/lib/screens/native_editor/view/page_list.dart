import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/scroll_behavior.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/selection_handle.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';
import 'package:typie/screens/native_editor/view/magnifier.dart';
import 'package:typie/screens/native_editor/view/page_item.dart';
import 'package:typie/screens/native_editor/view/selection_handle.dart';
import 'package:typie/screens/native_editor/view/title_subtitle_fields.dart';

const _pagePadding = 40.0;

class PageList extends HookWidget {
  const PageList({
    required this.editor,
    required this.layout,
    required this.cursor,
    required this.isFocused,
    required this.isSelecting,
    required this.lineHighlightEnabled,
    required this.renderVersion,
    required this.scrollController,
    required this.horizontalScrollController,
    required this.onOpenInput,
    required this.onClearFocus,
    required this.onCommitComposing,
    required this.onSelectionStart,
    required this.onSelectionEnd,
    required this.onLongPressStateChanged,
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    required this.onEnterDocument,
    required this.onTitleHeaderHeightChanged,
    required this.typewriterEnabled,
    required this.typewriterPosition,
    this.fromHandle,
    this.toHandle,
    this.onRenderComplete,
    super.key,
  });

  final NativeEditor editor;
  final LayoutInfo layout;
  final CursorInfo? cursor;
  final bool isFocused;
  final bool isSelecting;
  final bool lineHighlightEnabled;
  final Object? renderVersion;
  final ScrollController scrollController;
  final ScrollController horizontalScrollController;
  final VoidCallback onOpenInput;
  final VoidCallback onClearFocus;
  final VoidCallback onCommitComposing;
  final VoidCallback onSelectionStart;
  final VoidCallback onSelectionEnd;
  final ValueChanged<bool> onLongPressStateChanged;
  final String title;
  final String subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;
  final VoidCallback onEnterDocument;
  final ValueChanged<double> onTitleHeaderHeightChanged;
  final bool typewriterEnabled;
  final double typewriterPosition;
  final SelectionHandleInfo? fromHandle;
  final SelectionHandleInfo? toHandle;
  final VoidCallback? onRenderComplete;

  @override
  Widget build(BuildContext context) {
    final titleFieldsKey = useMemoized(GlobalKey.new);
    final titleHeight = useState<double>(0);
    final cumulativeHeightsRef = useRef<List<double>>([0]);
    final layoutRef = useRef(layout);
    final cumulativeHeights = useMemoized(() {
      final heights = <double>[0];
      for (var i = 0; i < layout.pageCount; i++) {
        final pageHeight = layout.pageHeights.elementAtOrNull(i) ?? 0.0;
        final isLast = i == layout.pageCount - 1;
        final gap = layout.isPaginated && !isLast ? pageGap : 0.0;
        heights.add(heights.last + pageHeight + gap);
      }
      return heights;
    }, [layout.pageHeights, layout.pageCount, layout.isPaginated]);

    cumulativeHeightsRef.value = cumulativeHeights;
    layoutRef.value = layout;

    (int pageIdx, double localY) getPageAtPosition(double y) {
      final currentLayout = layoutRef.value;
      final currentHeights = cumulativeHeightsRef.value;
      final topPadding = currentLayout.isPaginated ? _pagePadding : 0.0;
      final bottomPadding = currentLayout.isPaginated ? 0.0 : _pagePadding;
      final headerHeight = titleHeight.value + topPadding + bottomPadding;
      final absoluteY = y + scrollController.offset;

      if (absoluteY < headerHeight) {
        return (-1, absoluteY);
      }

      final adjustedY = absoluteY - headerHeight;

      var low = 0;
      var high = currentHeights.length - 1;
      while (low < high) {
        final mid = (low + high) ~/ 2;
        if (currentHeights[mid] <= adjustedY) {
          low = mid + 1;
        } else {
          high = mid;
        }
      }

      final pageIdx = (low - 1).clamp(0, currentLayout.pageCount - 1);
      final localY = adjustedY - currentHeights[pageIdx];
      return (pageIdx, localY);
    }

    final longPressPosition = useState<Offset?>(null);
    final handleDragPosition = useState<Offset?>(null);
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

          final currentOffset = scrollController.offset;
          final newOffset = (currentOffset + verticalDirection.value * scrollSpeed).clamp(
            0.0,
            scrollController.position.maxScrollExtent,
          );

          if (newOffset != currentOffset) {
            scrollController.jumpTo(newOffset);
            scrolledY = verticalDirection.value > 0
                ? viewHeight -
                      edgeThreshold +
                      (newOffset >= scrollController.position.maxScrollExtent ? edgeThreshold : 0)
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

          final horizontalPadding = layoutRef.value.isPaginated ? _pagePadding : 0.0;
          final pointerX = scrolledX + horizontalScrollController.offset - horizontalPadding;

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
      if (fromHandle == null && toHandle == null) {
        handleDragPosition.value = null;
        draggingHandleType.value = null;
        dragAnchorHandle.value = null;
        stopAutoScroll();
      }
      return null;
    }, [fromHandle, toHandle]);

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewWidth = constraints.maxWidth;
        final viewHeight = constraints.maxHeight;

        final horizontalPadding = layout.isPaginated ? _pagePadding : 0.0;

        double getPointerX(double localX) {
          return localX + horizontalScrollController.offset - horizontalPadding;
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
          final scrollOffset = scrollController.hasClients ? scrollController.offset : 0.0;
          final hScrollOffset = horizontalScrollController.hasClients ? horizontalScrollController.offset : 0.0;
          final topPadding = layout.isPaginated ? _pagePadding : 0.0;
          final bottomPadding = layout.isPaginated ? 0.0 : _pagePadding;
          final titleHeaderHeight = titleHeight.value + topPadding + bottomPadding;
          final pageTopOffset = titleHeaderHeight + cumulativeHeights[handle.pageIdx];
          final y = pageTopOffset + handle.y - scrollOffset;
          final x = horizontalPadding + handle.x - hScrollOffset;
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
          onSelectionStart();

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
          onSelectionEnd();
        }

        final contentWidth = layout.pageWidth + horizontalPadding * 2;
        final needsHorizontalScroll = contentWidth > viewWidth;
        final horizontalPhysics = isSelecting || !needsHorizontalScroll
            ? const NeverScrollableScrollPhysics()
            : const _NonGestureBouncingScrollPhysics();

        final topPadding = layout.isPaginated ? _pagePadding : 0.0;
        final bottomPadding = layout.isPaginated ? 0.0 : _pagePadding;
        final titleAreaHeight = titleHeight.value + topPadding + bottomPadding;
        final defaultBottomPadding = layout.isPaginated ? _pagePadding : 200.0;
        final contentBottomPadding = EditorScrollBehavior.calculateTypewriterBottomPadding(
          defaultPadding: defaultBottomPadding,
          typewriterEnabled: typewriterEnabled,
          typewriterPosition: typewriterPosition,
          viewportHeight: scrollController.hasClients ? scrollController.position.viewportDimension : viewHeight,
          layout: layout,
          cursor: cursor,
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
                controller: scrollController,
                physics: isSelecting ? const NeverScrollableScrollPhysics() : const _NonGestureBouncingScrollPhysics(),
                padding: EdgeInsets.only(
                  left: horizontalPadding,
                  right: horizontalPadding,
                  bottom: contentBottomPadding,
                ),
                child: Column(
                  children: [
                    SizedBox(height: titleAreaHeight),
                    for (var i = 0; i < layout.pageCount; i++)
                      _PageSlot(
                        key: ValueKey(i),
                        scrollController: scrollController,
                        viewHeight: viewHeight,
                        pageTop: titleAreaHeight + cumulativeHeights[i],
                        pageBottom: titleAreaHeight + cumulativeHeights[i + 1],
                        pageIndex: i,
                        editor: editor,
                        renderVersion: renderVersion,
                        layout: layout,
                        cursor: cursor,
                        isFocused: isFocused,
                        lineHighlightEnabled: lineHighlightEnabled,
                        onRenderComplete: onRenderComplete,
                      ),
                  ],
                ),
              ),
            ),
          ),
        );

        final titleFields = _MeasuredTitleFields(
          key: titleFieldsKey,
          title: title,
          subtitle: subtitle,
          onTitleChanged: onTitleChanged,
          onSubtitleChanged: onSubtitleChanged,
          titleFocusNode: titleFocusNode,
          subtitleFocusNode: subtitleFocusNode,
          onEnterDocument: onEnterDocument,
          pageWidth: viewWidth,
          onHeightChanged: (height) {
            if (titleHeight.value != height) {
              titleHeight.value = height;
              final topPadding = layout.isPaginated ? _pagePadding : 0.0;
              final bottomPadding = layout.isPaginated ? 0.0 : _pagePadding;
              onTitleHeaderHeightChanged(height + topPadding + bottomPadding);
            }
          },
          onFieldTap: onClearFocus,
        );

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

        void dispatchTap(Offset localPosition) {
          final (pageIdx, localY) = getPageAtPosition(localPosition.dy);

          if (pageIdx < 0) {
            return;
          }

          onCommitComposing();
          onOpenInput();

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

          lastTapTime.value = now;
          lastTapPosition.value = localPosition;

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
            onCommitComposing();
            longPressPosition.value = details.localPosition;
            onLongPressStateChanged(true);
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
            onLongPressStateChanged(false);
          },
          onPanDown: (details) {
            if (isSelecting) {
              return;
            }
            if (scrollController.hasClients) {
              scrollController.position.hold(() {});
            }
            if (horizontalScrollController.hasClients) {
              horizontalScrollController.position.hold(() {});
            }
          },
          onPanStart: (details) {
            if (isSelecting) {
              return;
            }
            if (scrollController.hasClients) {
              verticalDrag.value = scrollController.position.drag(details, () {
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
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              listView,
              Positioned(
                top: 0,
                left: 0,
                right: 0,
                child: AnimatedBuilder(
                  animation: scrollController,
                  builder: (context, child) {
                    final offset = scrollController.hasClients ? scrollController.offset : 0.0;
                    return Transform.translate(offset: Offset(0, -offset), child: child);
                  },
                  child: titleFields,
                ),
              ),
              if (longPressPosition.value != null || handleDragPosition.value != null)
                EditorMagnifier(
                  position: handleDragPosition.value ?? longPressPosition.value!,
                  focalPoint: handleDragPosition.value ?? longPressPosition.value!,
                  pageSize: Size(layout.pageWidth, viewHeight),
                ),
            ],
          ),
        );

        return Stack(
          clipBehavior: Clip.none,
          children: [
            gestureDetector,
            ListenableBuilder(
              listenable: Listenable.merge([scrollController, horizontalScrollController]),
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
                        child: buildSelectionHandle(fromHandle!, SelectionHandleType.from),
                      ),
                    if (isFocused && toHandle != null && toPos != null)
                      Positioned(
                        left: toPos.dx,
                        top: toPos.dy,
                        child: buildSelectionHandle(toHandle!, SelectionHandleType.to),
                      ),
                  ],
                );
              },
            ),
          ],
        );
      },
    );
  }
}

class _MeasuredTitleFields extends StatefulWidget {
  const _MeasuredTitleFields({
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    required this.onEnterDocument,
    required this.pageWidth,
    required this.onHeightChanged,
    this.onFieldTap,
    super.key,
  });

  final String title;
  final String subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;
  final VoidCallback onEnterDocument;
  final double pageWidth;
  final ValueChanged<double> onHeightChanged;
  final VoidCallback? onFieldTap;

  @override
  State<_MeasuredTitleFields> createState() => _MeasuredTitleFieldsState();
}

class _MeasuredTitleFieldsState extends State<_MeasuredTitleFields> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _measureHeight());
  }

  @override
  void didUpdateWidget(_MeasuredTitleFields oldWidget) {
    super.didUpdateWidget(oldWidget);
    WidgetsBinding.instance.addPostFrameCallback((_) => _measureHeight());
  }

  void _measureHeight() {
    final renderBox = context.findRenderObject() as RenderBox?;
    if (renderBox != null && renderBox.hasSize) {
      widget.onHeightChanged(renderBox.size.height);
    }
  }

  @override
  Widget build(BuildContext context) {
    return TitleSubtitleFields(
      title: widget.title,
      subtitle: widget.subtitle,
      onTitleChanged: widget.onTitleChanged,
      onSubtitleChanged: widget.onSubtitleChanged,
      titleFocusNode: widget.titleFocusNode,
      subtitleFocusNode: widget.subtitleFocusNode,
      onEnterDocument: widget.onEnterDocument,
      pageWidth: widget.pageWidth,
      onFieldTap: widget.onFieldTap,
    );
  }
}

class _PageSlot extends HookWidget {
  const _PageSlot({
    required this.scrollController,
    required this.viewHeight,
    required this.pageTop,
    required this.pageBottom,
    required this.pageIndex,
    required this.editor,
    required this.renderVersion,
    required this.layout,
    required this.cursor,
    required this.isFocused,
    required this.lineHighlightEnabled,
    this.onRenderComplete,
    super.key,
  });

  final ScrollController scrollController;
  final double viewHeight;
  final double pageTop;
  final double pageBottom;
  final int pageIndex;
  final NativeEditor editor;
  final Object? renderVersion;
  final LayoutInfo layout;
  final CursorInfo? cursor;
  final bool isFocused;
  final bool lineHighlightEnabled;
  final VoidCallback? onRenderComplete;

  bool _computeVisibility() {
    if (!scrollController.hasClients) {
      return true;
    }
    final scrollOffset = scrollController.offset;
    const cacheExtent = 200.0;
    final viewTop = scrollOffset - cacheExtent;
    final viewBottom = scrollOffset + viewHeight + cacheExtent;
    return pageBottom >= viewTop && pageTop <= viewBottom;
  }

  @override
  Widget build(BuildContext context) {
    final visible = useState(_computeVisibility());

    useEffect(() {
      void updateVisibility() {
        final nowVisible = _computeVisibility();
        if (nowVisible != visible.value) {
          visible.value = nowVisible;
        }
      }

      scrollController.addListener(updateVisibility);
      updateVisibility();
      return () => scrollController.removeListener(updateVisibility);
    }, [scrollController, pageTop, pageBottom, viewHeight]);

    final slotHeight = pageBottom - pageTop;

    if (!visible.value) {
      return SizedBox(height: slotHeight);
    }

    final isLast = pageIndex == layout.pageCount - 1;
    final gap = layout.isPaginated && !isLast ? pageGap : 0.0;
    final pageHeight = layout.pageHeights.elementAtOrNull(pageIndex);
    final pageCursor = cursor?.pageIdx == pageIndex ? cursor : null;
    final layoutMode = layout.layoutMode;
    final margins = layoutMode is PaginatedLayoutMode ? layoutMode : null;

    return PageItem(
      pageIndex: pageIndex,
      editor: editor,
      renderVersion: renderVersion,
      bottomGap: gap,
      pageWidth: layout.pageWidth,
      pageHeight: pageHeight,
      cursorInfo: pageCursor,
      isFocused: isFocused,
      lineHighlightEnabled: lineHighlightEnabled,
      isPaginated: layout.isPaginated,
      pageMarginTop: margins?.pageMarginTop ?? 0,
      pageMarginBottom: margins?.pageMarginBottom ?? 0,
      pageMarginLeft: margins?.pageMarginLeft ?? 0,
      pageMarginRight: margins?.pageMarginRight ?? 0,
      onRenderComplete: onRenderComplete,
    );
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
