import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/controller/scroll_behavior.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';
import 'package:typie/screens/native_editor/view/magnifier.dart';
import 'package:typie/screens/native_editor/view/page_item.dart';
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

  @override
  Widget build(BuildContext context) {
    final titleFieldsKey = useMemoized(GlobalKey.new);

    double getTitleHeaderHeight() {
      final renderBox = titleFieldsKey.currentContext?.findRenderObject() as RenderBox?;
      return renderBox?.size.height ?? 0;
    }

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

    (int pageIdx, double localY) getPageAtPosition(double y) {
      final topPadding = layout.isPaginated ? _pagePadding : 0.0;
      final bottomPadding = layout.isPaginated ? 0.0 : _pagePadding;
      final titleHeight = getTitleHeaderHeight() + topPadding + bottomPadding;
      final absoluteY = y + scrollController.offset;

      if (absoluteY < titleHeight) {
        return (-1, absoluteY);
      }

      final adjustedY = absoluteY - titleHeight;

      var low = 0;
      var high = cumulativeHeights.length - 1;
      while (low < high) {
        final mid = (low + high) ~/ 2;
        if (cumulativeHeights[mid] <= adjustedY) {
          low = mid + 1;
        } else {
          high = mid;
        }
      }

      final pageIdx = (low - 1).clamp(0, layout.pageCount - 1);
      final localY = adjustedY - cumulativeHeights[pageIdx];
      return (pageIdx, localY);
    }

    final longPressPosition = useState<Offset?>(null);
    final lastTapTime = useRef<DateTime?>(null);
    final lastTapPosition = useRef<Offset?>(null);
    final autoScrollTimer = useRef<Timer?>(null);

    const edgeThreshold = 60.0;
    const minScrollSpeed = 4.0;
    const maxScrollSpeed = 16.0;
    final verticalEdgeDistance = useRef<double>(0);
    final horizontalEdgeDistance = useRef<double>(0);
    final verticalDirection = useRef<double>(0);
    final horizontalDirection = useRef<double>(0);
    final autoScrollViewSize = useRef<Size>(Size.zero);

    void stopAutoScroll() {
      autoScrollTimer.value?.cancel();
      autoScrollTimer.value = null;
      verticalDirection.value = 0;
      horizontalDirection.value = 0;
    }

    void startAutoScroll() {
      if (autoScrollTimer.value != null) {
        return;
      }
      autoScrollTimer.value = Timer.periodic(const Duration(milliseconds: 16), (_) {
        final viewHeight = autoScrollViewSize.value.height;
        final viewWidth = autoScrollViewSize.value.width;
        var scrolledY = longPressPosition.value?.dy ?? 0;
        var scrolledX = longPressPosition.value?.dx ?? 0;

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

        final pos = longPressPosition.value;
        if (pos != null) {
          final (pageIdx, localY) = getPageAtPosition(scrolledY);

          if (pageIdx < 0) {
            return;
          }

          final horizontalPadding = layout.isPaginated ? _pagePadding : 0.0;
          final pointerX = scrolledX + horizontalScrollController.offset - horizontalPadding;

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
      });
    }

    useEffect(() => stopAutoScroll, const []);

    final titleHeight = useState<double>(0);

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewWidth = constraints.maxWidth;
        final viewHeight = constraints.maxHeight;

        final horizontalPadding = layout.isPaginated ? _pagePadding : 0.0;

        double getPointerX(double localX) {
          return localX + horizontalScrollController.offset - horizontalPadding;
        }

        final contentWidth = layout.pageWidth + horizontalPadding * 2;
        final needsHorizontalScroll = contentWidth > viewWidth;
        final horizontalPhysics = isSelecting || !needsHorizontalScroll
            ? const NeverScrollableScrollPhysics()
            : const BouncingScrollPhysics();

        final listView = ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: SingleChildScrollView(
            controller: horizontalScrollController,
            scrollDirection: Axis.horizontal,
            physics: horizontalPhysics,
            child: SizedBox(
              width: contentWidth,
              child: ListView.builder(
                controller: scrollController,
                padding: EdgeInsets.only(
                  left: horizontalPadding,
                  right: horizontalPadding,
                  bottom: layout.isPaginated ? _pagePadding : 200,
                ),
                itemCount: layout.pageCount + 1,
                cacheExtent: 1000,
                physics: isSelecting ? const NeverScrollableScrollPhysics() : const AlwaysScrollableScrollPhysics(),
                itemBuilder: (context, index) {
                  if (index == 0) {
                    final topPadding = layout.isPaginated ? _pagePadding : 0.0;
                    final bottomPadding = layout.isPaginated ? 0.0 : _pagePadding;
                    return SizedBox(height: titleHeight.value + topPadding + bottomPadding);
                  }

                  final pageIndex = index - 1;
                  final isLast = pageIndex == layout.pageCount - 1;
                  final gap = layout.isPaginated && !isLast ? pageGap : 0.0;
                  final pageHeight = layout.pageHeights.elementAtOrNull(pageIndex);
                  final pageCursor = cursor?.pageIdx == pageIndex ? cursor : null;

                  final layoutMode = layout.layoutMode;
                  final margins = layoutMode is PaginatedLayoutMode ? layoutMode : null;

                  return PageItem(
                    key: ValueKey(pageIndex),
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
                  );
                },
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
        );

        return GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: (details) {
            final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);

            if (pageIdx < 0) {
              return;
            }

            onOpenInput();

            final now = DateTime.now();
            final prevTime = lastTapTime.value;
            final prevPosition = lastTapPosition.value;

            var clickCount = 1;
            if (prevTime != null && prevPosition != null) {
              final timeDiff = now.difference(prevTime).inMilliseconds;
              final distance = (details.localPosition - prevPosition).distance;
              if (timeDiff < 300 && distance < 20) {
                clickCount = 2;
              }
            }

            lastTapTime.value = now;
            lastTapPosition.value = details.localPosition;

            final pointerX = getPointerX(details.localPosition.dx);
            editor.dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'clickCount': clickCount,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
          },
          onTapUp: (details) {
            final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);

            if (pageIdx < 0) {
              return;
            }

            final pointerX = getPointerX(details.localPosition.dx);
            editor.dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
          },
          onLongPressStart: (details) {
            longPressPosition.value = details.localPosition;
            onLongPressStateChanged(true);
          },
          onLongPressMoveUpdate: (details) {
            final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);
            longPressPosition.value = details.localPosition;
            autoScrollViewSize.value = Size(viewWidth, viewHeight);

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

            final y = details.localPosition.dy;
            final x = details.localPosition.dx;

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
          },
          onLongPressEnd: (details) {
            longPressPosition.value = null;
            stopAutoScroll();
            onLongPressStateChanged(false);
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
              if (longPressPosition.value != null)
                EditorMagnifier(
                  position: longPressPosition.value!,
                  focalPoint: longPressPosition.value!,
                  pageSize: Size(layout.pageWidth, viewHeight),
                ),
            ],
          ),
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
    );
  }
}
