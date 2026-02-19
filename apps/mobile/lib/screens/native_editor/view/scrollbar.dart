import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/scroll.dart';
import 'package:typie/services/preference.dart';

const _hideDelay = Duration(milliseconds: 1000);
const _minThumbSize = 30.0;
const _trackPadding = 2.0;
const _trackWidth = 12.0;
const _thumbWidth = 8.0;
const _indicatorHeight = 24.0;
const _indicatorGap = 14.0;
const _thumbHitPadding = 20.0;
const _thumbHitWidth = 44.0;

enum _ScrollVisibilitySource { user, auto }

class EditorScrollbar extends HookWidget {
  const EditorScrollbar({required this.viewHeight, required this.viewWidth, this.suppressShowOnScroll, super.key});

  final double viewHeight;
  final double viewWidth;
  final ValueNotifier<bool>? suppressShowOnScroll;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final pref = useService<Pref>();
    final state = useListenable(scope.controller);

    final verticalScrollController = scope.verticalScrollController;
    final horizontalScrollController = scope.horizontalScrollController;
    final layout = state.state.layout!;
    final pages = state.state.pages;
    final cursor = state.state.cursor;
    useValueListenable(scope.titleAreaHeight);
    final typewriterEnabled = pref.typewriterEnabled;
    final typewriterPosition = pref.typewriterPosition;

    final isVisible = useState(false);
    final visibleScrollSource = useState(_ScrollVisibilitySource.user);
    final isDraggingV = useState(false);
    final isDraggingH = useState(false);
    final hideTimer = useRef<Timer?>(null);

    final rebuildTrigger = useState(0);

    final safePadding = MediaQuery.paddingOf(context);

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

    void showTemporarily({_ScrollVisibilitySource source = _ScrollVisibilitySource.auto}) {
      isVisible.value = true;
      visibleScrollSource.value = source;
      if (!isDraggingV.value && !isDraggingH.value) {
        scheduleHide();
      }
    }

    useEffect(() {
      void onScroll() {
        if (!isDraggingV.value) {
          rebuildTrigger.value++;
        }
        final source = (suppressShowOnScroll?.value ?? false)
            ? _ScrollVisibilitySource.auto
            : _ScrollVisibilitySource.user;
        showTemporarily(source: source);
      }

      void onHorizontalScroll() {
        if (!isDraggingH.value) {
          rebuildTrigger.value++;
        }
        final source = (suppressShowOnScroll?.value ?? false)
            ? _ScrollVisibilitySource.auto
            : _ScrollVisibilitySource.user;
        showTemporarily(source: source);
      }

      verticalScrollController.addListener(onScroll);
      horizontalScrollController.addListener(onHorizontalScroll);

      WidgetsBinding.instance.addPostFrameCallback((_) {
        rebuildTrigger.value++;
      });

      return () {
        verticalScrollController.removeListener(onScroll);
        horizontalScrollController.removeListener(onHorizontalScroll);
        cancelHideTimer();
      };
    }, [verticalScrollController, horizontalScrollController]);

    final _ = rebuildTrigger.value;
    final isUserScrollVisible = visibleScrollSource.value == _ScrollVisibilitySource.user;

    final geometry = scope.geometry;
    final hasVerticalClients =
        verticalScrollController.hasSingleClient && verticalScrollController.position.hasContentDimensions;

    double calculateTotalContentHeight() {
      final viewportHeight = hasVerticalClients ? verticalScrollController.position.viewportDimension : viewHeight;
      return geometry.totalContentHeight(
        viewportHeight: viewportHeight,
        cursor: cursor,
        typewriterEnabled: typewriterEnabled,
        typewriterPosition: typewriterPosition,
      );
    }

    final hasHorizontalScroll =
        horizontalScrollController.hasSingleClient &&
        horizontalScrollController.position.hasContentDimensions &&
        horizontalScrollController.position.maxScrollExtent > 0;

    final actualViewHeight = hasVerticalClients ? verticalScrollController.position.viewportDimension : viewHeight;
    final actualViewWidth = hasHorizontalScroll ? horizontalScrollController.position.viewportDimension : viewWidth;

    final totalContentHeight = calculateTotalContentHeight();
    final calculatedMaxScrollExtent = math.max<double>(0, totalContentHeight - actualViewHeight);
    final hasVerticalScroll = calculatedMaxScrollExtent > 0;

    if (!hasVerticalScroll && !hasHorizontalScroll) {
      return const SizedBox.shrink();
    }

    final scrollOffset = hasVerticalClients
        ? verticalScrollController.offset.clamp(0.0, calculatedMaxScrollExtent)
        : 0.0;
    final maxScrollExtent = calculatedMaxScrollExtent;
    final viewportDimension = actualViewHeight;

    final horizontalScrollOffset = hasHorizontalScroll ? horizontalScrollController.offset : 0.0;
    final horizontalMaxScrollExtent = hasHorizontalScroll ? horizontalScrollController.position.maxScrollExtent : 0.0;
    final horizontalViewportDimension = hasHorizontalScroll
        ? horizontalScrollController.position.viewportDimension
        : viewWidth;

    final safeTop = safePadding.top;
    final safeBottom = safePadding.bottom;
    final trackHeight =
        actualViewHeight - _trackPadding * 2 - safeTop - safeBottom - (hasHorizontalScroll ? _trackWidth : 0);
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
        actualViewWidth -
        _trackPadding * 2 -
        safeLeft -
        safeRight -
        safeBottom * 2 -
        (hasVerticalScroll ? _trackWidth : 0);
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

      for (var i = 0; i < pages.length; i++) {
        final pageTop = cumHeight;
        final pageHeight = pages.elementAtOrNull(i)?.height ?? 0.0;
        final pageBottom = cumHeight + pageHeight;
        cumHeight = pageBottom + geometry.gapAfterPage(i);

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
      if (layout is PaginatedLayout) {
        final mostVisiblePage = calculateMostVisiblePage();
        return '${mostVisiblePage + 1}/${pages.length}';
      }
      return '${(scrollRatioV * 100).round()}%';
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
              opacity: isVisible.value || isDraggingV.value ? (isUserScrollVisible ? 1.0 : 0.65) : 0.0,
              duration: const Duration(milliseconds: 300),
              child: IgnorePointer(
                child: _VerticalScrollbarThumb(
                  thumbTop: thumbTop,
                  thumbHeight: thumbHeight,
                  isDragging: isDraggingV.value,
                  isUserScrollVisible: isUserScrollVisible,
                ),
              ),
            ),
          ),
        if (hasVerticalScroll)
          Positioned(
            right: 0,
            top: safeTop + thumbTop - _thumbHitPadding,
            width: _thumbHitWidth,
            height: thumbHeight + _thumbHitPadding * 2,
            child: IgnorePointer(
              ignoring: !isVisible.value && !isDraggingV.value,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTapDown: (details) {
                  isDraggingV.value = true;
                  dragStartThumbTop.value = thumbTop;
                  dragStartY.value = details.globalPosition.dy;
                  cancelHideTimer();
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onTapUp: (_) {
                  isDraggingV.value = false;
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onTapCancel: () {},
                onPanStart: (details) {
                  isDraggingV.value = true;
                  dragStartThumbTop.value = thumbTop;
                  dragStartY.value = details.globalPosition.dy;
                  cancelHideTimer();
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onPanUpdate: (details) {
                  if (!isDraggingV.value || !verticalScrollController.hasSingleClient) {
                    return;
                  }
                  final currentMaxExtent = verticalScrollController.position.maxScrollExtent;
                  final deltaY = details.globalPosition.dy - dragStartY.value;
                  final newThumbTop = dragStartThumbTop.value + deltaY;
                  final ratio = ((newThumbTop - _trackPadding) / (trackHeight - thumbHeight)).clamp(0.0, 1.0);
                  verticalScrollController.jumpTo(ratio * currentMaxExtent);
                  rebuildTrigger.value++;
                },
                onPanEnd: (_) {
                  isDraggingV.value = false;
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onPanCancel: () {
                  isDraggingV.value = false;
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
              ),
            ),
          ),
        if (hasVerticalScroll && isUserScrollVisible)
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
              opacity: isVisible.value || isDraggingH.value ? (isUserScrollVisible ? 1.0 : 0.65) : 0.0,
              duration: const Duration(milliseconds: 300),
              child: IgnorePointer(
                child: _HorizontalScrollbarThumb(
                  thumbLeft: thumbLeft,
                  thumbWidth: thumbWidthH,
                  isDragging: isDraggingH.value,
                  isUserScrollVisible: isUserScrollVisible,
                ),
              ),
            ),
          ),
        if (hasHorizontalScroll)
          Positioned(
            left: safeLeft + safeBottom + thumbLeft - _thumbHitPadding,
            bottom: 0,
            width: thumbWidthH + _thumbHitPadding * 2,
            height: _thumbHitWidth,
            child: IgnorePointer(
              ignoring: !isVisible.value && !isDraggingH.value,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTapDown: (details) {
                  isDraggingH.value = true;
                  dragStartThumbLeft.value = thumbLeft;
                  dragStartX.value = details.globalPosition.dx;
                  cancelHideTimer();
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onTapUp: (_) {
                  isDraggingH.value = false;
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onTapCancel: () {},
                onPanStart: (details) {
                  isDraggingH.value = true;
                  dragStartThumbLeft.value = thumbLeft;
                  dragStartX.value = details.globalPosition.dx;
                  cancelHideTimer();
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onPanUpdate: (details) {
                  if (!isDraggingH.value || !horizontalScrollController.hasSingleClient) {
                    return;
                  }
                  final currentMaxExtent = horizontalScrollController.position.maxScrollExtent;
                  final deltaX = details.globalPosition.dx - dragStartX.value;
                  final newThumbLeft = dragStartThumbLeft.value + deltaX;
                  final ratio = ((newThumbLeft - _trackPadding) / (trackWidthH - thumbWidthH)).clamp(0.0, 1.0);
                  horizontalScrollController.jumpTo(ratio * currentMaxExtent);
                  rebuildTrigger.value++;
                },
                onPanEnd: (_) {
                  isDraggingH.value = false;
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
                onPanCancel: () {
                  isDraggingH.value = false;
                  showTemporarily(source: _ScrollVisibilitySource.user);
                },
              ),
            ),
          ),
      ],
    );
  }
}

class _VerticalScrollbarThumb extends StatelessWidget {
  const _VerticalScrollbarThumb({
    required this.thumbTop,
    required this.thumbHeight,
    required this.isDragging,
    required this.isUserScrollVisible,
  });

  final double thumbTop;
  final double thumbHeight;
  final bool isDragging;
  final bool isUserScrollVisible;

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
              color: isDragging
                  ? Colors.black.withValues(alpha: isUserScrollVisible ? 0.8 : 0.45)
                  : Colors.black.withValues(alpha: isUserScrollVisible ? 0.5 : 0.22),
              borderRadius: BorderRadius.circular(4),
            ),
          ),
        ),
      ],
    );
  }
}

class _HorizontalScrollbarThumb extends StatelessWidget {
  const _HorizontalScrollbarThumb({
    required this.thumbLeft,
    required this.thumbWidth,
    required this.isDragging,
    required this.isUserScrollVisible,
  });

  final double thumbLeft;
  final double thumbWidth;
  final bool isDragging;
  final bool isUserScrollVisible;

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
              color: isDragging
                  ? Colors.black.withValues(alpha: isUserScrollVisible ? 0.8 : 0.45)
                  : Colors.black.withValues(alpha: isUserScrollVisible ? 0.5 : 0.22),
              borderRadius: BorderRadius.circular(4),
            ),
          ),
        ),
      ],
    );
  }
}
