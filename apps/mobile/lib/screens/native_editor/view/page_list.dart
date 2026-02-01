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
    required this.viewKeyboardHeight,
    required this.onOpenInput,
    required this.onSelectionStart,
    required this.onSelectionEnd,
    required this.title,
    required this.subtitle,
    required this.onTitleChanged,
    required this.onSubtitleChanged,
    required this.titleFocusNode,
    required this.subtitleFocusNode,
    required this.onEnterDocument,
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
  final double viewKeyboardHeight;
  final VoidCallback onOpenInput;
  final VoidCallback onSelectionStart;
  final VoidCallback onSelectionEnd;
  final String title;
  final String subtitle;
  final ValueChanged<String> onTitleChanged;
  final ValueChanged<String> onSubtitleChanged;
  final FocusNode titleFocusNode;
  final FocusNode subtitleFocusNode;
  final VoidCallback onEnterDocument;

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
      final titleHeight = getTitleHeaderHeight();
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
    final edgeDistance = useRef<double>(0);

    void stopAutoScroll() {
      autoScrollTimer.value?.cancel();
      autoScrollTimer.value = null;
    }

    void startAutoScroll(double direction, double viewHeight) {
      stopAutoScroll();
      autoScrollTimer.value = Timer.periodic(const Duration(milliseconds: 16), (_) {
        final proximity = 1.0 - (edgeDistance.value / edgeThreshold).clamp(0.0, 1.0);
        final scrollSpeed = minScrollSpeed + proximity * (maxScrollSpeed - minScrollSpeed);

        final currentOffset = scrollController.offset;
        final newOffset = (currentOffset + direction * scrollSpeed).clamp(
          0.0,
          scrollController.position.maxScrollExtent,
        );

        if (newOffset == currentOffset) {
          return;
        }

        scrollController.jumpTo(newOffset);

        final pos = longPressPosition.value;
        if (pos != null) {
          final scrolledY = direction > 0
              ? viewHeight -
                    edgeThreshold +
                    (newOffset >= scrollController.position.maxScrollExtent ? edgeThreshold : 0)
              : newOffset.clamp(0.0, edgeThreshold);
          final (pageIdx, localY) = getPageAtPosition(scrolledY);
          editor
            ..dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': pos.dx,
              'y': localY,
              'clickCount': 1,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            })
            ..dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIdx,
              'x': pos.dx,
              'y': localY,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
        }
      });
    }

    useEffect(() => stopAutoScroll, const []);

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewHeight = constraints.maxHeight;

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

            editor.dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': details.localPosition.dx,
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

            editor.dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIdx,
              'x': details.localPosition.dx,
              'y': localY,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
          },
          onLongPressStart: (details) {
            longPressPosition.value = details.localPosition;
          },
          onLongPressMoveUpdate: (details) {
            final (pageIdx, localY) = getPageAtPosition(details.localPosition.dy);
            longPressPosition.value = details.localPosition;

            if (pageIdx >= 0) {
              editor
                ..dispatch({
                  'type': 'pointerDown',
                  'pageIdx': pageIdx,
                  'x': details.localPosition.dx,
                  'y': localY,
                  'clickCount': 1,
                  'button': 'primary',
                  'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
                })
                ..dispatch({
                  'type': 'pointerUp',
                  'pageIdx': pageIdx,
                  'x': details.localPosition.dx,
                  'y': localY,
                  'button': 'primary',
                  'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
                });
            }

            final y = details.localPosition.dy;
            if (y < edgeThreshold) {
              edgeDistance.value = y;
              startAutoScroll(-1, viewHeight);
            } else if (y > viewHeight - edgeThreshold) {
              edgeDistance.value = viewHeight - y;
              startAutoScroll(1, viewHeight);
            } else {
              stopAutoScroll();
            }
          },
          onLongPressEnd: (details) {
            longPressPosition.value = null;
            stopAutoScroll();
          },
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              ListView.builder(
                controller: scrollController,
                padding: EdgeInsets.only(bottom: viewKeyboardHeight),
                itemCount: layout.pageCount + 1,
                cacheExtent: 1000,
                physics: isSelecting ? const NeverScrollableScrollPhysics() : const AlwaysScrollableScrollPhysics(),
                itemBuilder: (context, index) {
                  if (index == 0) {
                    return TitleSubtitleFields(
                      key: titleFieldsKey,
                      title: title,
                      subtitle: subtitle,
                      onTitleChanged: onTitleChanged,
                      onSubtitleChanged: onSubtitleChanged,
                      titleFocusNode: titleFocusNode,
                      subtitleFocusNode: subtitleFocusNode,
                      onEnterDocument: onEnterDocument,
                      pageWidth: layout.pageWidth,
                    );
                  }

                  final pageIndex = index - 1;
                  final isLast = pageIndex == layout.pageCount - 1;
                  final gap = layout.isPaginated && !isLast ? pageGap : 0.0;
                  final pageHeight = layout.pageHeights.elementAtOrNull(pageIndex);
                  final pageCursor = cursor?.pageIdx == pageIndex ? cursor : null;

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
                  );
                },
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
