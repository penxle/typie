import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';
import 'package:typie/services/preference.dart';

const _hideDelay = Duration(milliseconds: 1000);
const _minThumbSize = 30.0;
const _trackPadding = 2.0;
const _trackWidth = 12.0;
const _thumbWidth = 8.0;
const _indicatorHeight = 24.0;
const _indicatorGap = 14.0;

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
    final toolbarScope = NativeEditorToolbarScope.of(context);

    final verticalScrollController = scope.verticalScrollController;
    final horizontalScrollController = scope.horizontalScrollController;
    final isKeyboardVisible = useValueListenable(toolbarScope.isKeyboardVisible);
    final isEditorFocused = useValueListenable(toolbarScope.isEditorFocused);
    final layout = state.state.layout!;
    final pages = state.state.pages;
    final cursor = state.state.cursor;
    useValueListenable(scope.titleAreaHeight);
    useValueListenable(scope.displayZoom);
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
    final verticalPosition = resolveScrollPosition(verticalScrollController);
    final hasVerticalClients = verticalPosition != null && verticalPosition.hasContentDimensions;
    final horizontalMetrics = resolveHorizontalScrollMetrics(
      controller: horizontalScrollController,
      contentWidth: geometry.contentWidth,
      fallbackViewportDimension: viewWidth,
    );

    double calculateTotalContentHeight() {
      final viewportHeight = hasVerticalClients ? verticalPosition.viewportDimension : viewHeight;
      return geometry.totalContentHeight(
        viewportHeight: viewportHeight,
        cursor: cursor,
        typewriterEnabled: typewriterEnabled,
        typewriterPosition: typewriterPosition,
      );
    }

    final hasHorizontalScroll = horizontalMetrics.expectsScrollableContent;

    final actualViewHeight = hasVerticalClients ? verticalPosition.viewportDimension : viewHeight;
    final actualViewWidth = hasHorizontalScroll ? horizontalMetrics.viewportDimension : viewWidth;

    final totalContentHeight = calculateTotalContentHeight();
    final calculatedMaxScrollExtent = math.max<double>(0, totalContentHeight - actualViewHeight);
    final hasVerticalScroll = calculatedMaxScrollExtent > 0;

    if (!hasVerticalScroll && !hasHorizontalScroll) {
      return const SizedBox.shrink();
    }

    final scrollOffset = hasVerticalClients ? verticalPosition.pixels.clamp(0.0, calculatedMaxScrollExtent) : 0.0;
    final maxScrollExtent = calculatedMaxScrollExtent;
    final viewportDimension = actualViewHeight;

    final horizontalScrollOffset = hasHorizontalScroll ? horizontalMetrics.scrollOffset : 0.0;
    final horizontalMaxScrollExtent = hasHorizontalScroll
        ? (horizontalMetrics.hasScrollablePositionExtent
              ? horizontalMetrics.maxScrollExtent
              : horizontalMetrics.expectedMaxScrollExtent)
        : 0.0;
    final horizontalViewportDimension = hasHorizontalScroll ? horizontalMetrics.viewportDimension : viewWidth;

    final safeTop = safePadding.top;
    final toolbarVisible = isEditorFocused;
    final safeBottom = (!isKeyboardVisible && !toolbarVisible) ? safePadding.bottom : 0.0;
    final rawTrackHeight =
        actualViewHeight - _trackPadding * 2 - safeTop - safeBottom - (hasHorizontalScroll ? _trackWidth : 0);
    final trackHeight = math.max<double>(0, rawTrackHeight);
    final thumbRatio = viewportDimension > 0 ? viewportDimension / (viewportDimension + maxScrollExtent) : 1.0;
    final thumbHeight = math.min(trackHeight, math.max(_minThumbSize, thumbRatio * trackHeight));
    final thumbTravelV = math.max<double>(0, trackHeight - thumbHeight);
    final scrollRatioV = maxScrollExtent > 0 ? (scrollOffset / maxScrollExtent).clamp(0.0, 1.0) : 0.0;
    final thumbTop = _trackPadding + scrollRatioV * thumbTravelV;

    final safeLeft = safePadding.left;
    final safeRight = safePadding.right;
    final rawTrackWidthH =
        actualViewWidth -
        _trackPadding * 2 -
        safeLeft -
        safeRight -
        safeBottom * 2 -
        (hasVerticalScroll ? _trackWidth : 0);
    final trackWidthH = math.max<double>(0, rawTrackWidthH);
    final horizontalThumbRatio = horizontalViewportDimension > 0
        ? horizontalViewportDimension / (horizontalViewportDimension + horizontalMaxScrollExtent)
        : 1.0;
    final thumbWidthH = math.min(trackWidthH, math.max(_minThumbSize, horizontalThumbRatio * trackWidthH));
    final thumbTravelH = math.max<double>(0, trackWidthH - thumbWidthH);
    final scrollRatioH = horizontalMaxScrollExtent > 0
        ? (horizontalScrollOffset / horizontalMaxScrollExtent).clamp(0.0, 1.0)
        : 0.0;
    final thumbLeft = _trackPadding + scrollRatioH * thumbTravelH;

    int calculateMostVisiblePage() {
      final offset = scrollOffset.clamp(0.0, maxScrollExtent);
      final viewport = viewportDimension;

      var cumHeight = 0.0;
      var mostVisible = 0;
      var maxVisibility = 0.0;

      for (var i = 0; i < pages.length; i++) {
        final pageTop = cumHeight;
        final pageHeight = geometry.pageHeightAt(i);
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
            top: safeTop + thumbTop,
            width: _trackWidth,
            height: thumbHeight,
            child: AnimatedOpacity(
              opacity: isVisible.value || isDraggingV.value ? (isUserScrollVisible ? 1.0 : 0.65) : 0.0,
              duration: const Duration(milliseconds: 300),
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
                    final dragVerticalPosition = resolveScrollPosition(verticalScrollController);
                    if (!isDraggingV.value ||
                        dragVerticalPosition == null ||
                        !dragVerticalPosition.hasContentDimensions) {
                      return;
                    }
                    final currentMaxExtent = dragVerticalPosition.maxScrollExtent;
                    final deltaY = details.globalPosition.dy - dragStartY.value;
                    final newThumbTop = dragStartThumbTop.value + deltaY;
                    final ratio = thumbTravelV > 0
                        ? ((newThumbTop - _trackPadding) / thumbTravelV).clamp(0.0, 1.0)
                        : 0.0;
                    dragVerticalPosition.jumpTo(ratio * currentMaxExtent);
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
                  child: Padding(
                    padding: const EdgeInsets.only(right: _trackPadding),
                    child: Align(
                      alignment: Alignment.centerRight,
                      child: AnimatedContainer(
                        duration: const Duration(milliseconds: 100),
                        width: _thumbWidth,
                        height: thumbHeight,
                        decoration: BoxDecoration(
                          color: isDraggingV.value
                              ? context.colors.surfaceInverse.withValues(alpha: isUserScrollVisible ? 0.8 : 0.45)
                              : context.colors.surfaceInverse.withValues(alpha: isUserScrollVisible ? 0.5 : 0.22),
                          borderRadius: BorderRadius.circular(4),
                        ),
                      ),
                    ),
                  ),
                ),
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
                    color: context.colors.surfaceInverse.withValues(alpha: 0.65),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Center(
                    child: Text(
                      getDisplayText(),
                      style: TextStyle(
                        color: context.colors.textInverse,
                        fontSize: 11,
                        fontFeatures: const [FontFeature.tabularFigures()],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        if (hasHorizontalScroll)
          Positioned(
            left: safeLeft + safeBottom + thumbLeft,
            bottom: 0,
            width: thumbWidthH,
            height: _trackWidth,
            child: AnimatedOpacity(
              opacity: isVisible.value || isDraggingH.value ? (isUserScrollVisible ? 1.0 : 0.65) : 0.0,
              duration: const Duration(milliseconds: 300),
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
                    final dragHorizontalMetrics = resolveHorizontalScrollMetrics(
                      controller: horizontalScrollController,
                      contentWidth: geometry.contentWidth,
                      fallbackViewportDimension: viewWidth,
                    );
                    final dragHorizontalPosition = dragHorizontalMetrics.activePosition;
                    if (!isDraggingH.value ||
                        dragHorizontalPosition == null ||
                        !dragHorizontalMetrics.canScrollHorizontally) {
                      return;
                    }
                    final currentMaxExtent = dragHorizontalPosition.maxScrollExtent;
                    final deltaX = details.globalPosition.dx - dragStartX.value;
                    final newThumbLeft = dragStartThumbLeft.value + deltaX;
                    final ratio = thumbTravelH > 0
                        ? ((newThumbLeft - _trackPadding) / thumbTravelH).clamp(0.0, 1.0)
                        : 0.0;
                    dragHorizontalPosition.jumpTo(ratio * currentMaxExtent);
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
                  child: Padding(
                    padding: const EdgeInsets.only(bottom: _trackPadding),
                    child: Align(
                      alignment: Alignment.bottomCenter,
                      child: AnimatedContainer(
                        duration: const Duration(milliseconds: 100),
                        width: thumbWidthH,
                        height: _thumbWidth,
                        decoration: BoxDecoration(
                          color: isDraggingH.value
                              ? context.colors.surfaceInverse.withValues(alpha: isUserScrollVisible ? 0.8 : 0.45)
                              : context.colors.surfaceInverse.withValues(alpha: isUserScrollVisible ? 0.5 : 0.22),
                          borderRadius: BorderRadius.circular(4),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}
