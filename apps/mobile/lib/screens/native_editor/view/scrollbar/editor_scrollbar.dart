import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/controller/scroll_behavior.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

const _hideDelay = Duration(milliseconds: 1000);
const _minThumbSize = 30.0;
const _trackPadding = 2.0;
const _trackWidth = 12.0;
const _thumbWidth = 8.0;
const _indicatorHeight = 24.0;
const _indicatorGap = 14.0;
const _thumbHitPadding = 8.0;

class EditorScrollbar extends HookWidget {
  const EditorScrollbar({
    required this.scrollController,
    required this.horizontalScrollController,
    required this.layout,
    required this.viewHeight,
    required this.viewWidth,
    required this.titleHeaderHeight,
    this.suppressShowOnScroll,
    super.key,
  });

  final ScrollController scrollController;
  final ScrollController horizontalScrollController;
  final LayoutInfo layout;
  final double viewHeight;
  final double viewWidth;
  final double titleHeaderHeight;
  final ValueNotifier<bool>? suppressShowOnScroll;

  @override
  Widget build(BuildContext context) {
    final isVisible = useState(false);
    final isDraggingV = useState(false);
    final isDraggingH = useState(false);
    final hideTimer = useRef<Timer?>(null);

    final rebuildTrigger = useState(0);

    final safePadding = MediaQuery.of(context).padding;

    final dragStartThumbTop = useRef<double>(0);
    final dragStartY = useRef<double>(0);
    final dragStartThumbLeft = useRef<double>(0);
    final dragStartX = useRef<double>(0);

    void cancelHideTimer() {
      hideTimer.value?.cancel();
      hideTimer.value = null;
    }

    void scheduleHide() {
      cancelHideTimer();
      hideTimer.value = Timer(_hideDelay, () {
        if (!isDraggingV.value && !isDraggingH.value) {
          isVisible.value = false;
        }
      });
    }

    void showTemporarily() {
      isVisible.value = true;
      if (!isDraggingV.value && !isDraggingH.value) {
        scheduleHide();
      }
    }

    void triggerRebuild() {
      rebuildTrigger.value++;
    }

    useEffect(() {
      void onScroll() {
        if (!isDraggingV.value) {
          triggerRebuild();
        }
        if (suppressShowOnScroll?.value != true) {
          showTemporarily();
        }
      }

      void onHorizontalScroll() {
        if (!isDraggingH.value) {
          triggerRebuild();
        }
        if (suppressShowOnScroll?.value != true) {
          showTemporarily();
        }
      }

      scrollController.addListener(onScroll);
      horizontalScrollController.addListener(onHorizontalScroll);

      WidgetsBinding.instance.addPostFrameCallback((_) {
        triggerRebuild();
      });

      return () {
        scrollController.removeListener(onScroll);
        horizontalScrollController.removeListener(onHorizontalScroll);
        cancelHideTimer();
      };
    }, [scrollController, horizontalScrollController]);

    final _ = rebuildTrigger.value;

    double calculateTotalContentHeight() {
      var total = titleHeaderHeight;
      for (var i = 0; i < layout.pageCount; i++) {
        total += layout.pageHeights.elementAtOrNull(i) ?? 0.0;
        if (layout.isPaginated && i < layout.pageCount - 1) {
          total += pageGap;
        }
      }
      final bottomPadding = layout.isPaginated ? 40.0 : 200.0;
      return total + bottomPadding;
    }

    final totalContentHeight = calculateTotalContentHeight();
    final calculatedMaxScrollExtent = math.max<double>(0, totalContentHeight - viewHeight);

    final hasVerticalClients = scrollController.hasClients && scrollController.position.hasContentDimensions;
    final hasVerticalScroll = calculatedMaxScrollExtent > 0;
    final hasHorizontalScroll =
        horizontalScrollController.hasClients &&
        horizontalScrollController.position.hasContentDimensions &&
        horizontalScrollController.position.maxScrollExtent > 0;

    if (!hasVerticalScroll && !hasHorizontalScroll) {
      return const SizedBox.shrink();
    }

    final scrollOffset = hasVerticalClients ? scrollController.offset.clamp(0.0, calculatedMaxScrollExtent) : 0.0;
    final maxScrollExtent = calculatedMaxScrollExtent;
    final viewportDimension = viewHeight;

    final horizontalScrollOffset = hasHorizontalScroll ? horizontalScrollController.offset : 0.0;
    final horizontalMaxScrollExtent = hasHorizontalScroll ? horizontalScrollController.position.maxScrollExtent : 0.0;
    final horizontalViewportDimension = hasHorizontalScroll
        ? horizontalScrollController.position.viewportDimension
        : viewWidth;

    final safeTop = safePadding.top;
    final safeBottom = safePadding.bottom;
    final trackHeight = viewHeight - _trackPadding * 2 - safeTop - safeBottom - (hasHorizontalScroll ? _trackWidth : 0);
    final thumbRatio = viewportDimension > 0 ? viewportDimension / (viewportDimension + maxScrollExtent) : 1.0;
    final thumbHeight = math.max(_minThumbSize, thumbRatio * trackHeight);
    final scrollRatioV = maxScrollExtent > 0 ? (scrollOffset / maxScrollExtent).clamp(0.0, 1.0) : 0.0;
    final thumbTop = (_trackPadding + scrollRatioV * (trackHeight - thumbHeight)).clamp(
      _trackPadding,
      _trackPadding + trackHeight - thumbHeight,
    );

    final safeLeft = safePadding.left;
    final safeRight = safePadding.right;
    final trackWidthH =
        viewWidth - _trackPadding * 2 - safeLeft - safeRight - safeBottom * 2 - (hasVerticalScroll ? _trackWidth : 0);
    final horizontalThumbRatio = horizontalViewportDimension > 0
        ? horizontalViewportDimension / (horizontalViewportDimension + horizontalMaxScrollExtent)
        : 1.0;
    final thumbWidthH = math.max(_minThumbSize, horizontalThumbRatio * trackWidthH);
    final scrollRatioH = horizontalMaxScrollExtent > 0
        ? (horizontalScrollOffset / horizontalMaxScrollExtent).clamp(0.0, 1.0)
        : 0.0;
    final thumbLeft = (_trackPadding + scrollRatioH * (trackWidthH - thumbWidthH)).clamp(
      _trackPadding,
      _trackPadding + trackWidthH - thumbWidthH,
    );

    int calculateMostVisiblePage() {
      final offset = scrollOffset.clamp(0.0, maxScrollExtent);
      final viewport = viewportDimension;

      var cumHeight = 0.0;
      var mostVisible = 0;
      var maxVisibility = 0.0;

      for (var i = 0; i < layout.pageCount; i++) {
        final pageTop = cumHeight;
        final pageHeight = layout.pageHeights.elementAtOrNull(i) ?? 0.0;
        final pageBottom = cumHeight + pageHeight;
        final isLast = i == layout.pageCount - 1;
        cumHeight = pageBottom + (layout.isPaginated && !isLast ? pageGap : 0);

        final visibleTop = math.max(pageTop, offset);
        final visibleBottom = math.min(pageBottom, offset + viewport);
        final visibility = math.max<double>(0, visibleBottom - visibleTop);

        if (visibility > maxVisibility) {
          maxVisibility = visibility;
          mostVisible = i;
        }
      }
      return mostVisible;
    }

    String getDisplayText() {
      if (layout.isPaginated) {
        final mostVisiblePage = calculateMostVisiblePage();
        return '${mostVisiblePage + 1}/${layout.pageCount}';
      }
      return '${(scrollRatioV * 100).round()}%';
    }

    bool isInThumbAreaV(double localY) {
      final hitTop = thumbTop - _thumbHitPadding;
      final hitBottom = thumbTop + thumbHeight + _thumbHitPadding;
      return localY >= hitTop && localY <= hitBottom;
    }

    bool isInThumbAreaH(double localX) {
      final hitLeft = thumbLeft - _thumbHitPadding;
      final hitRight = thumbLeft + thumbWidthH + _thumbHitPadding;
      return localX >= hitLeft && localX <= hitRight;
    }

    void handleTrackTapV(double localY) {
      if (!scrollController.hasClients) {
        return;
      }
      if (isInThumbAreaV(localY)) {
        return;
      }
      final clickY = localY - _trackPadding;
      final ratio = ((clickY - thumbHeight / 2) / (trackHeight - thumbHeight)).clamp(0.0, 1.0);
      scrollController.jumpTo(ratio * maxScrollExtent);
    }

    void handleTrackTapH(double localX) {
      if (!horizontalScrollController.hasClients) {
        return;
      }
      if (isInThumbAreaH(localX)) {
        return;
      }
      final clickX = localX - _trackPadding;
      final ratio = ((clickX - thumbWidthH / 2) / (trackWidthH - thumbWidthH)).clamp(0.0, 1.0);
      horizontalScrollController.jumpTo(ratio * horizontalMaxScrollExtent);
    }

    return Stack(
      children: [
        if (hasVerticalScroll)
          Positioned(
            right: 0,
            top: safeTop,
            bottom: safeBottom + (hasHorizontalScroll ? _trackWidth : 0),
            width: _trackWidth,
            child: AnimatedOpacity(
              opacity: isVisible.value || isDraggingV.value ? 1.0 : 0.0,
              duration: const Duration(milliseconds: 300),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTapDown: (details) {
                  if (isInThumbAreaV(details.localPosition.dy)) {
                    isDraggingV.value = true;
                    dragStartThumbTop.value = thumbTop;
                    dragStartY.value = details.localPosition.dy;
                    cancelHideTimer();
                    isVisible.value = true;
                  }
                },
                onTapUp: (details) {
                  if (isDraggingV.value) {
                    isDraggingV.value = false;
                    scheduleHide();
                  } else {
                    handleTrackTapV(details.localPosition.dy);
                  }
                },
                onTapCancel: () {
                  // Tap cancelled - likely transitioning to pan, keep state
                },
                onPanStart: (details) {
                  if (!isDraggingV.value && isInThumbAreaV(details.localPosition.dy)) {
                    isDraggingV.value = true;
                    dragStartThumbTop.value = thumbTop;
                    dragStartY.value = details.localPosition.dy;
                    cancelHideTimer();
                    isVisible.value = true;
                  }
                },
                onPanUpdate: (details) {
                  if (!isDraggingV.value || !scrollController.hasClients) {
                    return;
                  }
                  final currentMaxExtent = scrollController.position.maxScrollExtent;
                  final deltaY = details.localPosition.dy - dragStartY.value;
                  final newThumbTop = dragStartThumbTop.value + deltaY;
                  final ratio = ((newThumbTop - _trackPadding) / (trackHeight - thumbHeight)).clamp(0.0, 1.0);
                  scrollController.jumpTo(ratio * currentMaxExtent);
                  triggerRebuild();
                },
                onPanEnd: (_) {
                  isDraggingV.value = false;
                  scheduleHide();
                },
                onPanCancel: () {
                  isDraggingV.value = false;
                  scheduleHide();
                },
                child: _VerticalScrollbarThumb(
                  thumbTop: thumbTop,
                  thumbHeight: thumbHeight,
                  isDragging: isDraggingV.value,
                ),
              ),
            ),
          ),
        if (hasVerticalScroll)
          Positioned(
            right: _indicatorGap,
            top: safeTop + thumbTop + thumbHeight / 2 - _indicatorHeight / 2,
            child: AnimatedOpacity(
              opacity: isVisible.value || isDraggingV.value ? 1.0 : 0.0,
              duration: const Duration(milliseconds: 300),
              child: IgnorePointer(
                child: Container(
                  height: _indicatorHeight,
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: Colors.black.withValues(alpha: 0.65),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Center(
                    child: Text(
                      getDisplayText(),
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 11,
                        fontFeatures: [FontFeature.tabularFigures()],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        if (hasHorizontalScroll)
          Positioned(
            left: safeLeft + safeBottom,
            right: safeRight + safeBottom + (hasVerticalScroll ? _trackWidth : 0),
            bottom: 0,
            height: _trackWidth,
            child: AnimatedOpacity(
              opacity: isVisible.value || isDraggingH.value ? 1.0 : 0.0,
              duration: const Duration(milliseconds: 300),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTapDown: (details) {
                  if (isInThumbAreaH(details.localPosition.dx)) {
                    isDraggingH.value = true;
                    dragStartThumbLeft.value = thumbLeft;
                    dragStartX.value = details.localPosition.dx;
                    cancelHideTimer();
                    isVisible.value = true;
                  }
                },
                onTapUp: (details) {
                  if (isDraggingH.value) {
                    isDraggingH.value = false;
                    scheduleHide();
                  } else {
                    handleTrackTapH(details.localPosition.dx);
                  }
                },
                onTapCancel: () {
                  // Tap cancelled - likely transitioning to pan, keep state
                },
                onPanStart: (details) {
                  if (!isDraggingH.value && isInThumbAreaH(details.localPosition.dx)) {
                    isDraggingH.value = true;
                    dragStartThumbLeft.value = thumbLeft;
                    dragStartX.value = details.localPosition.dx;
                    cancelHideTimer();
                    isVisible.value = true;
                  }
                },
                onPanUpdate: (details) {
                  if (!isDraggingH.value || !horizontalScrollController.hasClients) {
                    return;
                  }
                  final currentMaxExtent = horizontalScrollController.position.maxScrollExtent;
                  final deltaX = details.localPosition.dx - dragStartX.value;
                  final newThumbLeft = dragStartThumbLeft.value + deltaX;
                  final ratio = ((newThumbLeft - _trackPadding) / (trackWidthH - thumbWidthH)).clamp(0.0, 1.0);
                  horizontalScrollController.jumpTo(ratio * currentMaxExtent);
                  triggerRebuild();
                },
                onPanEnd: (_) {
                  isDraggingH.value = false;
                  scheduleHide();
                },
                onPanCancel: () {
                  isDraggingH.value = false;
                  scheduleHide();
                },
                child: _HorizontalScrollbarThumb(
                  thumbLeft: thumbLeft,
                  thumbWidth: thumbWidthH,
                  isDragging: isDraggingH.value,
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _VerticalScrollbarThumb extends StatelessWidget {
  const _VerticalScrollbarThumb({required this.thumbTop, required this.thumbHeight, required this.isDragging});

  final double thumbTop;
  final double thumbHeight;
  final bool isDragging;

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        Positioned(
          right: _trackPadding,
          top: thumbTop,
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 100),
            width: _thumbWidth,
            height: thumbHeight,
            decoration: BoxDecoration(
              color: isDragging ? Colors.black.withValues(alpha: 0.8) : Colors.black.withValues(alpha: 0.5),
              borderRadius: BorderRadius.circular(4),
            ),
          ),
        ),
      ],
    );
  }
}

class _HorizontalScrollbarThumb extends StatelessWidget {
  const _HorizontalScrollbarThumb({required this.thumbLeft, required this.thumbWidth, required this.isDragging});

  final double thumbLeft;
  final double thumbWidth;
  final bool isDragging;

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        Positioned(
          left: thumbLeft,
          bottom: _trackPadding,
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 100),
            width: thumbWidth,
            height: _thumbWidth,
            decoration: BoxDecoration(
              color: isDragging ? Colors.black.withValues(alpha: 0.8) : Colors.black.withValues(alpha: 0.5),
              borderRadius: BorderRadius.circular(4),
            ),
          ),
        ),
      ],
    );
  }
}
