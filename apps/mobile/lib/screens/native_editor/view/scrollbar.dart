import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';
import 'package:typie/services/preference.dart';

const _hideDelay = Duration(milliseconds: 1500);
const _indicatorHideDelay = Duration(milliseconds: 300);
const _longPressDuration = Duration(milliseconds: 100);
const _minThumbSize = 30.0;
const _trackPadding = 2.0;
const _trackWidth = 12.0;
const _thumbWidth = 6.0;
const _activeThumbWidth = 10.0;
const _longPressHitExpansion = 16.0;
const _indicatorHeight = 24.0;
const _indicatorGap = 8.0;

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
    final currentStateCursor = state.state.cursor;
    final presentedViewport = useValueListenable(scope.presentedViewport);
    final currentRenderedCursor = presentedViewport.cursor;
    final currentStateRenderVersion = state.state.renderVersion;
    final hasAlignedRenderedCursor =
        currentRenderedCursor != null && presentedViewport.renderVersion == currentStateRenderVersion;
    final alignedPresentedViewport = hasAlignedRenderedCursor ? presentedViewport : null;

    final cursor = hasAlignedRenderedCursor ? currentRenderedCursor : currentStateCursor;
    useValueListenable(scope.titleAreaHeight);
    useValueListenable(scope.displayZoom);
    final typewriterEnabled = pref.typewriterEnabled;
    final typewriterPosition = pref.typewriterPosition;

    final isVisible = useState(false);
    final isIndicatorVisible = useState(false);
    final visibleScrollSource = useState(_ScrollVisibilitySource.user);
    final isDraggingV = useState(false);
    final isDraggingH = useState(false);
    final hideTimer = useRef<Timer?>(null);
    final indicatorHideTimer = useRef<Timer?>(null);

    final rebuildTrigger = useState(0);

    final safePadding = MediaQuery.paddingOf(context);

    final dragStartThumbTop = useRef<double>(0);
    final dragStartY = useRef<double>(0);
    final dragStartThumbLeft = useRef<double>(0);
    final dragStartX = useRef<double>(0);

    void cancelHideTimer() {
      indicatorHideTimer.value?.cancel();
      indicatorHideTimer.value = null;
      hideTimer.value?.cancel();
      hideTimer.value = null;
    }

    void scheduleHide() {
      cancelHideTimer();
      indicatorHideTimer.value = Timer(_indicatorHideDelay, () {
        if (!isDraggingV.value && !isDraggingH.value) {
          isIndicatorVisible.value = false;
        }
      });
      hideTimer.value = Timer(_hideDelay, () {
        if (!isDraggingV.value && !isDraggingH.value) {
          isVisible.value = false;
          isIndicatorVisible.value = false;
        }
      });
    }

    void showTemporarily({_ScrollVisibilitySource source = _ScrollVisibilitySource.auto}) {
      isVisible.value = true;
      visibleScrollSource.value = source;
      isIndicatorVisible.value = source == _ScrollVisibilitySource.user;
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

    double calculateFallbackTotalContentHeight() {
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

    final fallbackTotalContentHeight = calculateFallbackTotalContentHeight();
    final fallbackMaxScrollExtent = math.max<double>(0, fallbackTotalContentHeight - actualViewHeight);
    final positionMaxScrollExtent = hasVerticalClients ? math.max<double>(0, verticalPosition.maxScrollExtent) : 0.0;
    final shouldUseTypewriterProjection = alignedPresentedViewport?.hasProjectedMetrics ?? false;

    late final double maxScrollExtent;
    late final double scrollOffset;
    late final double viewportDimension;

    if (hasVerticalClients && shouldUseTypewriterProjection) {
      maxScrollExtent = alignedPresentedViewport!.projectedMaxScrollExtent!;
      scrollOffset = alignedPresentedViewport.projectedScrollOffset!.clamp(0.0, maxScrollExtent);
      viewportDimension = alignedPresentedViewport.projectedViewportHeight!;
    } else if (hasVerticalClients) {
      maxScrollExtent = positionMaxScrollExtent;
      scrollOffset = verticalPosition.pixels.clamp(0.0, maxScrollExtent);
      viewportDimension = actualViewHeight;
    } else {
      maxScrollExtent = fallbackMaxScrollExtent;
      scrollOffset = 0.0;
      viewportDimension = actualViewHeight;
    }

    final hasVerticalScroll = hasScrollableExtent(maxScrollExtent);

    if (!hasVerticalScroll && !hasHorizontalScroll) {
      return const SizedBox.shrink();
    }

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
    final indicatorTop = safeTop + thumbTop + thumbHeight / 2 - _indicatorHeight / 2;
    const indicatorRight = _trackPadding + _activeThumbWidth + _indicatorGap;

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

    void triggerGrabHaptic() {
      unawaited(HapticFeedback.lightImpact());
    }

    void triggerReleaseHaptic() {
      unawaited(HapticFeedback.lightImpact());
    }

    void startVerticalDrag(double globalY) {
      if (!isDraggingV.value) {
        triggerGrabHaptic();
      }
      isDraggingV.value = true;
      dragStartThumbTop.value = thumbTop;
      dragStartY.value = globalY;
      cancelHideTimer();
      showTemporarily(source: _ScrollVisibilitySource.user);
    }

    void updateVerticalDrag(double globalY) {
      final dragVerticalPosition = resolveScrollPosition(verticalScrollController);
      if (!isDraggingV.value || dragVerticalPosition == null || !dragVerticalPosition.hasContentDimensions) {
        return;
      }
      final currentMaxExtent = dragVerticalPosition.maxScrollExtent;
      final deltaY = globalY - dragStartY.value;
      final newThumbTop = dragStartThumbTop.value + deltaY;
      final ratio = thumbTravelV > 0 ? ((newThumbTop - _trackPadding) / thumbTravelV).clamp(0.0, 1.0) : 0.0;
      dragVerticalPosition.jumpTo(ratio * currentMaxExtent);
      rebuildTrigger.value++;
    }

    void stopVerticalDrag() {
      if (!isDraggingV.value) {
        return;
      }
      triggerReleaseHaptic();
      isDraggingV.value = false;
      showTemporarily(source: _ScrollVisibilitySource.user);
    }

    void startHorizontalDrag(double globalX) {
      if (!isDraggingH.value) {
        triggerGrabHaptic();
      }
      isDraggingH.value = true;
      dragStartThumbLeft.value = thumbLeft;
      dragStartX.value = globalX;
      cancelHideTimer();
      showTemporarily(source: _ScrollVisibilitySource.user);
    }

    void updateHorizontalDrag(double globalX) {
      final dragHorizontalMetrics = resolveHorizontalScrollMetrics(
        controller: horizontalScrollController,
        contentWidth: geometry.contentWidth,
        fallbackViewportDimension: viewWidth,
      );
      final dragHorizontalPosition = dragHorizontalMetrics.activePosition;
      if (!isDraggingH.value || dragHorizontalPosition == null || !dragHorizontalMetrics.canScrollHorizontally) {
        return;
      }
      final currentMaxExtent = dragHorizontalPosition.maxScrollExtent;
      final deltaX = globalX - dragStartX.value;
      final newThumbLeft = dragStartThumbLeft.value + deltaX;
      final ratio = thumbTravelH > 0 ? ((newThumbLeft - _trackPadding) / thumbTravelH).clamp(0.0, 1.0) : 0.0;
      dragHorizontalPosition.jumpTo(ratio * currentMaxExtent);
      rebuildTrigger.value++;
    }

    void stopHorizontalDrag() {
      if (!isDraggingH.value) {
        return;
      }
      triggerReleaseHaptic();
      isDraggingH.value = false;
      showTemporarily(source: _ScrollVisibilitySource.user);
    }

    Widget buildLongPressDetector({
      required void Function(Offset globalPosition) onStart,
      required void Function(Offset globalPosition) onMove,
      required VoidCallback onEnd,
    }) {
      return RawGestureDetector(
        behavior: HitTestBehavior.opaque,
        gestures: <Type, GestureRecognizerFactory>{
          LongPressGestureRecognizer: GestureRecognizerFactoryWithHandlers<LongPressGestureRecognizer>(
            () => LongPressGestureRecognizer(duration: _longPressDuration, postAcceptSlopTolerance: double.infinity),
            (LongPressGestureRecognizer recognizer) {
              recognizer
                ..onLongPressStart = (LongPressStartDetails details) {
                  onStart(details.globalPosition);
                }
                ..onLongPressMoveUpdate = (LongPressMoveUpdateDetails details) {
                  onMove(details.globalPosition);
                }
                ..onLongPressEnd = (LongPressEndDetails details) {
                  onEnd();
                };
            },
          ),
        },
        child: const SizedBox.expand(),
      );
    }

    final canDirectInteractV = isVisible.value || isDraggingV.value;
    final canDirectInteractH = isVisible.value || isDraggingH.value;

    return Stack(
      children: [
        if (hasVerticalScroll)
          Positioned(
            right: 0,
            top: safeTop + thumbTop,
            width: _trackWidth + _longPressHitExpansion,
            height: thumbHeight,
            child: AnimatedOpacity(
              opacity: isVisible.value || isDraggingV.value ? (isUserScrollVisible ? 1.0 : 0.65) : 0.0,
              duration: const Duration(milliseconds: 300),
              child: Stack(
                fit: StackFit.expand,
                children: [
                  IgnorePointer(
                    ignoring: !canDirectInteractV,
                    child: buildLongPressDetector(
                      onStart: (globalPosition) {
                        startVerticalDrag(globalPosition.dy);
                      },
                      onMove: (globalPosition) {
                        updateVerticalDrag(globalPosition.dy);
                      },
                      onEnd: stopVerticalDrag,
                    ),
                  ),
                  Align(
                    alignment: Alignment.centerRight,
                    child: IgnorePointer(
                      ignoring: !canDirectInteractV,
                      child: SizedBox(
                        width: _trackWidth,
                        child: Listener(
                          onPointerCancel: (event) {
                            stopVerticalDrag();
                          },
                          child: GestureDetector(
                            behavior: HitTestBehavior.opaque,
                            onTapDown: (details) {
                              startVerticalDrag(details.globalPosition.dy);
                            },
                            onTapUp: (details) {
                              stopVerticalDrag();
                            },
                            onPanStart: (details) {
                              startVerticalDrag(details.globalPosition.dy);
                            },
                            onPanUpdate: (details) {
                              updateVerticalDrag(details.globalPosition.dy);
                            },
                            onPanEnd: (_) {
                              stopVerticalDrag();
                            },
                            onPanCancel: stopVerticalDrag,
                            child: Padding(
                              padding: const EdgeInsets.only(right: _trackPadding),
                              child: Align(
                                alignment: Alignment.centerRight,
                                child: SizedBox(
                                  height: thumbHeight,
                                  child: OverflowBox(
                                    alignment: Alignment.centerRight,
                                    minWidth: 0,
                                    maxWidth: _activeThumbWidth * 2,
                                    minHeight: thumbHeight,
                                    maxHeight: thumbHeight,
                                    child: AnimatedContainer(
                                      key: const ValueKey('native-editor-scrollbar-thumb-vertical'),
                                      duration: const Duration(milliseconds: 250),
                                      curve: Curves.easeInOutBack,
                                      width: isDraggingV.value ? _activeThumbWidth : _thumbWidth,
                                      height: double.infinity,
                                      decoration: BoxDecoration(
                                        color: isDraggingV.value
                                            ? context.colors.surfaceInverse.withValues(
                                                alpha: isUserScrollVisible ? 0.8 : 0.45,
                                              )
                                            : context.colors.surfaceInverse.withValues(
                                                alpha: isUserScrollVisible ? 0.5 : 0.22,
                                              ),
                                        borderRadius: BorderRadius.circular(4),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        if (hasVerticalScroll && isUserScrollVisible)
          Positioned(
            right: 0,
            top: indicatorTop,
            child: Padding(
              padding: const EdgeInsets.only(right: indicatorRight),
              child: AnimatedOpacity(
                opacity: isIndicatorVisible.value || isDraggingV.value ? 1.0 : 0.0,
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
          ),
        if (hasHorizontalScroll)
          Positioned(
            left: safeLeft + safeBottom + thumbLeft,
            bottom: 0,
            width: thumbWidthH,
            height: _trackWidth + _longPressHitExpansion,
            child: AnimatedOpacity(
              opacity: isVisible.value || isDraggingH.value ? (isUserScrollVisible ? 1.0 : 0.65) : 0.0,
              duration: const Duration(milliseconds: 300),
              child: Stack(
                fit: StackFit.expand,
                children: [
                  IgnorePointer(
                    ignoring: !canDirectInteractH,
                    child: buildLongPressDetector(
                      onStart: (globalPosition) {
                        startHorizontalDrag(globalPosition.dx);
                      },
                      onMove: (globalPosition) {
                        updateHorizontalDrag(globalPosition.dx);
                      },
                      onEnd: stopHorizontalDrag,
                    ),
                  ),
                  Align(
                    alignment: Alignment.bottomCenter,
                    child: IgnorePointer(
                      ignoring: !canDirectInteractH,
                      child: SizedBox(
                        height: _trackWidth,
                        child: Listener(
                          onPointerCancel: (event) {
                            stopHorizontalDrag();
                          },
                          child: GestureDetector(
                            behavior: HitTestBehavior.opaque,
                            onTapDown: (details) {
                              startHorizontalDrag(details.globalPosition.dx);
                            },
                            onTapUp: (details) {
                              stopHorizontalDrag();
                            },
                            onPanStart: (details) {
                              startHorizontalDrag(details.globalPosition.dx);
                            },
                            onPanUpdate: (details) {
                              updateHorizontalDrag(details.globalPosition.dx);
                            },
                            onPanEnd: (_) {
                              stopHorizontalDrag();
                            },
                            onPanCancel: stopHorizontalDrag,
                            child: Padding(
                              padding: const EdgeInsets.only(bottom: _trackPadding),
                              child: Align(
                                alignment: Alignment.bottomCenter,
                                child: SizedBox(
                                  width: thumbWidthH,
                                  child: OverflowBox(
                                    alignment: Alignment.bottomCenter,
                                    minWidth: thumbWidthH,
                                    maxWidth: thumbWidthH,
                                    minHeight: 0,
                                    maxHeight: _activeThumbWidth * 2,
                                    child: AnimatedContainer(
                                      key: const ValueKey('native-editor-scrollbar-thumb-horizontal'),
                                      duration: const Duration(milliseconds: 250),
                                      curve: Curves.easeInOutBack,
                                      width: double.infinity,
                                      height: isDraggingH.value ? _activeThumbWidth : _thumbWidth,
                                      decoration: BoxDecoration(
                                        color: isDraggingH.value
                                            ? context.colors.surfaceInverse.withValues(
                                                alpha: isUserScrollVisible ? 0.8 : 0.45,
                                              )
                                            : context.colors.surfaceInverse.withValues(
                                                alpha: isUserScrollVisible ? 0.5 : 0.22,
                                              ),
                                        borderRadius: BorderRadius.circular(4),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
      ],
    );
  }
}
