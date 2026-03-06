import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/screens/stats/stats_calculator.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/fader.dart';
import 'package:typie/widgets/tappable.dart';

class ActivityChart extends HookWidget {
  const ActivityChart({super.key, required this.characterCountChanges});

  final List<StatsCharacterCountChange> characterCountChanges;

  static const chartHeight = 100.0;
  static const xAxisAreaHeight = 24.0;
  static const barAnimationDuration = Duration(milliseconds: 360);
  static const tooltipFadeDuration = Duration(milliseconds: 140);
  static const tooltipScrollActivationDistance = 12.0;
  static const tooltipScrollVelocityThreshold = 700.0;

  @override
  Widget build(BuildContext context) {
    final showAdditions = useState(true);
    final showDeletions = useState(true);
    final tooltipData = useState<({int index, _DayData dayData})?>(null);
    final isTooltipShown = useState(false);
    final selectedIndex = useState<int?>(null);
    final tooltipTimer = useRef<Timer?>(null);

    final zoom = useState<double>(1);
    final viewportWidth = useState<double>(0);
    final scrollOffset = useState<double>(0);
    final canScrollLeft = useState(false);
    final canScrollRight = useState(false);
    final isPinching = useState(false);

    final pinchStartZoom = useRef<double>(1);
    final pinchStartOffset = useRef<double>(0);
    final pinchStartFocalX = useRef<double>(0);
    final pinchUpdateToken = useRef(0);
    final dragPointers = useRef(<int>{});
    final tooltipGesturePointer = useRef<int?>(null);
    final tooltipGestureLastTimeStamp = useRef<Duration?>(null);
    final tooltipGestureStartLocalPosition = useRef<Offset?>(null);
    final tooltipGestureVelocityTracker = useRef<VelocityTracker?>(null);
    final tooltipScrollDrag = useRef<Drag?>(null);
    final isTooltipScrollGesture = useRef(false);

    final scrollController = useScrollController();

    void updateScrollState() {
      if (!scrollController.hasClients) {
        scrollOffset.value = 0;
        canScrollLeft.value = false;
        canScrollRight.value = false;
        return;
      }

      if (isPinching.value) {
        final maxOffset = math.max(viewportWidth.value * zoom.value - viewportWidth.value, 0);
        canScrollLeft.value = scrollOffset.value > 0.5;
        canScrollRight.value = scrollOffset.value < maxOffset - 0.5;
        return;
      }

      final offset = scrollController.offset;
      scrollOffset.value = offset;
      canScrollLeft.value = offset > 0.5;
      canScrollRight.value = offset < scrollController.position.maxScrollExtent - 0.5;
    }

    void resetTooltipGesture({bool cancelScrollDrag = false}) {
      if (cancelScrollDrag) {
        tooltipScrollDrag.value?.cancel();
      }

      tooltipScrollDrag.value = null;
      tooltipGesturePointer.value = null;
      tooltipGestureLastTimeStamp.value = null;
      tooltipGestureStartLocalPosition.value = null;
      tooltipGestureVelocityTracker.value = null;
      isTooltipScrollGesture.value = false;
    }

    void beginTooltipGestureTracking(PointerEvent event) {
      tooltipGesturePointer.value = event.pointer;
      tooltipGestureLastTimeStamp.value = event.timeStamp;
      tooltipGestureStartLocalPosition.value = event.localPosition;
      tooltipGestureVelocityTracker.value = VelocityTracker.withKind(event.kind)
        ..addPosition(event.timeStamp, event.position);
    }

    useEffect(() {
      return () {
        tooltipScrollDrag.value?.cancel();
        tooltipTimer.value?.cancel();
      };
    }, []);

    useEffect(() {
      void onScroll() {
        updateScrollState();
      }

      scrollController.addListener(onScroll);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        onScroll();
      });

      return () {
        scrollController.removeListener(onScroll);
      };
    }, [scrollController]);

    final daysData = useMemoized(() => _generateDaysData(characterCountChanges), [characterCountChanges]);
    final xAxisLabels = useMemoized(() => _generateXAxisLabels(daysData, zoom.value), [daysData, zoom.value]);

    void showTooltip(int index, {required bool withHaptic}) {
      if (index < 0 || index >= daysData.length) {
        return;
      }

      tooltipTimer.value?.cancel();

      if (withHaptic && selectedIndex.value != index) {
        unawaited(HapticFeedback.selectionClick());
      }

      selectedIndex.value = index;
      tooltipData.value = (index: index, dayData: daysData[index]);
      isTooltipShown.value = true;
    }

    void hideTooltip() {
      tooltipTimer.value?.cancel();
      isTooltipShown.value = false;
      selectedIndex.value = null;
    }

    void hideAfterDelay(Duration delay) {
      tooltipTimer.value?.cancel();
      tooltipTimer.value = Timer(delay, hideTooltip);
    }

    void beginPinch(double focalX) {
      resetTooltipGesture(cancelScrollDrag: true);
      pinchStartZoom.value = zoom.value;
      pinchStartOffset.value = scrollController.hasClients ? scrollController.offset : scrollOffset.value;
      pinchStartFocalX.value = focalX;
      isPinching.value = true;

      hideTooltip();
    }

    void updatePinch(double scale) {
      final scaledZoom = pinchStartZoom.value * scale;
      final nextZoom = scaledZoom < 1 ? 1.0 : (scaledZoom > 4 ? 4.0 : scaledZoom);
      if ((nextZoom - zoom.value).abs() < 0.0005) {
        return;
      }

      final focalX = pinchStartFocalX.value < 0
          ? 0.0
          : (pinchStartFocalX.value > viewportWidth.value ? viewportWidth.value : pinchStartFocalX.value);
      final contentX = pinchStartOffset.value + focalX;
      final targetOffset = contentX * (nextZoom / pinchStartZoom.value) - focalX;
      final visualMaxOffset = math.max(viewportWidth.value * nextZoom - viewportWidth.value, 0);
      final clampedTargetOffset = targetOffset.clamp(0.0, visualMaxOffset).toDouble();

      zoom.value = nextZoom;
      scrollOffset.value = clampedTargetOffset;
      updateScrollState();
      pinchUpdateToken.value += 1;
      final updateToken = pinchUpdateToken.value;

      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (updateToken != pinchUpdateToken.value) {
          return;
        }

        if (!scrollController.hasClients) {
          return;
        }

        final maxOffset = scrollController.position.maxScrollExtent;
        final clampedOffset = clampedTargetOffset.clamp(0.0, maxOffset);
        if ((scrollController.offset - clampedOffset).abs() > 0.5) {
          scrollController.jumpTo(clampedOffset);
        }
        updateScrollState();
      });
    }

    void scrollByViewport(int direction) {
      if (!scrollController.hasClients) {
        return;
      }

      final maxExtent = scrollController.position.maxScrollExtent;
      final delta = math.max(viewportWidth.value * 0.75, 80);
      final target = (scrollController.offset + (delta * direction)).clamp(0.0, maxExtent);

      unawaited(
        scrollController.animateTo(target, duration: const Duration(milliseconds: 200), curve: Curves.easeOutCubic),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Text(
              '지난 3개월간의 기록',
              style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
            ),
          ],
        ),
        const SizedBox(height: 8),
        LayoutBuilder(
          builder: (context, constraints) {
            final chartWidth = constraints.maxWidth;

            if ((viewportWidth.value - chartWidth).abs() > 0.5) {
              WidgetsBinding.instance.addPostFrameCallback((_) {
                viewportWidth.value = chartWidth;

                if (!scrollController.hasClients) {
                  return;
                }

                final maxOffset = chartWidth * zoom.value - chartWidth;
                final double clampedMaxOffset = maxOffset > 0 ? maxOffset : 0;
                if (scrollController.offset > clampedMaxOffset) {
                  scrollController.jumpTo(clampedMaxOffset);
                }

                updateScrollState();
              });
            }

            final totalChartWidth = chartWidth * zoom.value;
            final barWidth = daysData.isEmpty ? 0.0 : totalChartWidth / daysData.length;
            final visualScrollDelta = isPinching.value && scrollController.hasClients
                ? (scrollController.offset - scrollOffset.value)
                : 0.0;
            final axisLabelStyle = TextStyle(fontSize: 12, color: context.colors.textFaint);
            final positionedXAxisLabels = _positionXAxisLabels(
              labels: xAxisLabels,
              barWidth: barWidth,
              chartWidth: totalChartWidth,
              textStyle: axisLabelStyle,
              textDirection: Directionality.of(context),
            );
            final visibleRange = _calculateVisibleRange(
              length: daysData.length,
              barWidth: barWidth,
              viewportWidth: chartWidth,
              scrollOffset: scrollOffset.value,
            );

            final maxVal = _maxValue(
              daysData,
              showAdditions.value,
              showDeletions.value,
              startIndex: visibleRange.start,
              endIndex: visibleRange.end,
            );

            final barHeights = [
              for (final dayData in daysData)
                _calculateBarHeights(
                  dayData: dayData,
                  showAdditions: showAdditions.value,
                  showDeletions: showDeletions.value,
                  chartHeight: chartHeight,
                  maxVal: maxVal,
                ),
            ];

            final isTooltipVisible = isTooltipShown.value;

            int getBarIndexAtPosition(Offset localPosition) {
              if (barWidth <= 0) {
                return -1;
              }

              final contentX = scrollOffset.value + localPosition.dx;
              final clampedContentX = contentX.clamp(0.0, math.max(totalChartWidth - 0.001, 0.0));
              final index = (clampedContentX / barWidth).floor();

              if (index < 0 || index >= daysData.length) {
                return -1;
              }

              return index;
            }

            void handleInteraction(Offset localPosition, {required bool withHaptic}) {
              final index = getBarIndexAtPosition(localPosition);
              if (index >= 0) {
                showTooltip(index, withHaptic: withHaptic);
              }
            }

            void handleInteractionEnd() {
              hideAfterDelay(const Duration(seconds: 1));
            }

            Offset horizontalDelta(Offset delta) => Offset(delta.dx, 0);

            DragUpdateDetails horizontalDragUpdateDetails(PointerMoveEvent event) {
              final delta = horizontalDelta(event.delta);

              return DragUpdateDetails(
                globalPosition: event.position,
                localPosition: event.localPosition,
                delta: delta,
                primaryDelta: delta.dx,
                sourceTimeStamp: event.timeStamp,
              );
            }

            void startTooltipScroll(PointerMoveEvent event) {
              if (!scrollController.hasClients) {
                return;
              }

              hideTooltip();
              isTooltipScrollGesture.value = true;
              tooltipScrollDrag.value?.cancel();
              tooltipScrollDrag.value = scrollController.position.drag(
                DragStartDetails(
                  globalPosition: event.position,
                  localPosition: event.localPosition,
                  sourceTimeStamp: event.timeStamp,
                ),
                () {
                  tooltipScrollDrag.value = null;
                  isTooltipScrollGesture.value = false;
                },
              );
              tooltipScrollDrag.value?.update(horizontalDragUpdateDetails(event));
            }

            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SizedBox(
                  height: chartHeight + xAxisAreaHeight,
                  child: Stack(
                    clipBehavior: Clip.none,
                    children: [
                      Positioned(
                        left: 0,
                        right: 0,
                        top: 0,
                        bottom: 0,
                        child: SingleChildScrollView(
                          controller: scrollController,
                          scrollDirection: Axis.horizontal,
                          physics: isTooltipVisible || isPinching.value ? const NeverScrollableScrollPhysics() : null,
                          child: SizedBox(
                            width: totalChartWidth,
                            child: Transform.translate(
                              offset: Offset(visualScrollDelta, 0),
                              child: Column(
                                children: [
                                  SizedBox(
                                    height: chartHeight,
                                    child: Stack(
                                      children: [
                                        Positioned.fill(
                                          child: Stack(
                                            children: [
                                              for (var i = 1; i <= 5; i++)
                                                Positioned(
                                                  left: 0,
                                                  right: 0,
                                                  bottom: i * 20.0,
                                                  child: Container(
                                                    height: 1,
                                                    color: context.theme.brightness == Brightness.dark
                                                        ? AppColors.dark.gray_700.withValues(alpha: 0.5)
                                                        : AppColors.gray_200.withValues(alpha: 0.5),
                                                  ),
                                                ),
                                            ],
                                          ),
                                        ),
                                        Row(
                                          crossAxisAlignment: CrossAxisAlignment.end,
                                          children: [
                                            for (var i = 0; i < daysData.length; i++)
                                              Expanded(
                                                child: SizedBox(
                                                  height: chartHeight,
                                                  child: Stack(
                                                    alignment: Alignment.bottomCenter,
                                                    children: [
                                                      if (daysData[i].deletions > 0 && showDeletions.value)
                                                        AnimatedPositioned(
                                                          duration: barAnimationDuration,
                                                          curve: Curves.easeOutCubic,
                                                          left: 1,
                                                          right: 1,
                                                          bottom: barHeights[i].additionHeight > 0
                                                              ? barHeights[i].additionHeight + 1
                                                              : 0,
                                                          child: AnimatedContainer(
                                                            duration: barAnimationDuration,
                                                            curve: Curves.easeOutCubic,
                                                            height: math.max(barHeights[i].deletionHeight, 1),
                                                            decoration: BoxDecoration(
                                                              color: context.theme.brightness == Brightness.dark
                                                                  ? AppColors.dark.gray_600
                                                                  : AppColors.gray_400,
                                                              borderRadius: BorderRadius.circular(1),
                                                            ),
                                                          ),
                                                        ),
                                                      if (daysData[i].additions > 0 && showAdditions.value)
                                                        AnimatedPositioned(
                                                          duration: barAnimationDuration,
                                                          curve: Curves.easeOutCubic,
                                                          left: 1,
                                                          right: 1,
                                                          bottom: 0,
                                                          child: AnimatedContainer(
                                                            duration: barAnimationDuration,
                                                            curve: Curves.easeOutCubic,
                                                            height: math.max(barHeights[i].additionHeight, 1),
                                                            decoration: BoxDecoration(
                                                              color: context.theme.brightness == Brightness.dark
                                                                  ? AppColors.dark.green_600
                                                                  : AppColors.green_400,
                                                              borderRadius: BorderRadius.circular(1),
                                                            ),
                                                          ),
                                                        ),
                                                      if (barHeights[i].effectiveTotal == 0)
                                                        Positioned(
                                                          left: 1,
                                                          right: 1,
                                                          bottom: 0,
                                                          child: Container(
                                                            height: 1,
                                                            decoration: BoxDecoration(
                                                              color: context.theme.brightness == Brightness.dark
                                                                  ? AppColors.dark.gray_700
                                                                  : AppColors.gray_200,
                                                              borderRadius: BorderRadius.circular(1),
                                                            ),
                                                          ),
                                                        ),
                                                      if (selectedIndex.value == i)
                                                        Positioned(
                                                          left: 0,
                                                          right: 0,
                                                          bottom: 0,
                                                          top: 0,
                                                          child: DecoratedBox(
                                                            decoration: BoxDecoration(
                                                              border: Border.all(color: context.colors.borderStrong),
                                                              borderRadius: BorderRadius.circular(2),
                                                            ),
                                                          ),
                                                        ),
                                                    ],
                                                  ),
                                                ),
                                              ),
                                          ],
                                        ),
                                      ],
                                    ),
                                  ),
                                  const SizedBox(height: 4),
                                  SizedBox(
                                    height: 20,
                                    child: Stack(
                                      children: [
                                        for (final label in positionedXAxisLabels)
                                          Positioned(
                                            left: label.left,
                                            child: Text(label.data.text, style: axisLabelStyle),
                                          ),
                                      ],
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ),
                      ),
                      Positioned(
                        top: 0,
                        left: 0,
                        right: 0,
                        height: chartHeight,
                        child: Listener(
                          behavior: HitTestBehavior.translucent,
                          onPointerDown: (event) {
                            dragPointers.value.add(event.pointer);

                            if (dragPointers.value.length > 1) {
                              resetTooltipGesture(cancelScrollDrag: true);
                              return;
                            }

                            if (!isPinching.value) {
                              beginTooltipGestureTracking(event);
                            }

                            if (tooltipData.value != null && !isPinching.value) {
                              handleInteraction(event.localPosition, withHaptic: false);
                            }
                          },
                          onPointerMove: (event) {
                            if (!dragPointers.value.contains(event.pointer)) {
                              return;
                            }

                            if (tooltipGesturePointer.value != event.pointer) {
                              return;
                            }

                            tooltipGestureVelocityTracker.value?.addPosition(event.timeStamp, event.position);

                            if (dragPointers.value.length != 1 || isPinching.value) {
                              return;
                            }

                            if (isTooltipScrollGesture.value) {
                              tooltipScrollDrag.value?.update(horizontalDragUpdateDetails(event));
                              tooltipGestureLastTimeStamp.value = event.timeStamp;
                              return;
                            }

                            if (tooltipData.value == null) {
                              return;
                            }

                            final lastTimeStamp = tooltipGestureLastTimeStamp.value;
                            final startLocalPosition = tooltipGestureStartLocalPosition.value;

                            if (lastTimeStamp != null && startLocalPosition != null) {
                              final elapsedMicroseconds = (event.timeStamp - lastTimeStamp).inMicroseconds;
                              if (elapsedMicroseconds > 0) {
                                final velocity =
                                    event.delta.dx.abs() * Duration.microsecondsPerSecond / elapsedMicroseconds;
                                final totalDistance = (event.localPosition.dx - startLocalPosition.dx).abs();

                                if (totalDistance >= tooltipScrollActivationDistance &&
                                    velocity >= tooltipScrollVelocityThreshold) {
                                  startTooltipScroll(event);
                                  tooltipGestureLastTimeStamp.value = event.timeStamp;
                                  return;
                                }
                              }
                            }

                            if (tooltipData.value != null) {
                              handleInteraction(event.localPosition, withHaptic: true);
                            }

                            tooltipGestureLastTimeStamp.value = event.timeStamp;
                          },
                          onPointerUp: (event) {
                            dragPointers.value.remove(event.pointer);

                            if (tooltipGesturePointer.value != event.pointer) {
                              return;
                            }

                            tooltipGestureVelocityTracker.value?.addPosition(event.timeStamp, event.position);

                            if (isTooltipScrollGesture.value) {
                              final velocity =
                                  tooltipGestureVelocityTracker.value?.getVelocity().pixelsPerSecond.dx ?? 0.0;
                              tooltipScrollDrag.value?.end(
                                DragEndDetails(
                                  velocity: Velocity(pixelsPerSecond: Offset(velocity, 0)),
                                  primaryVelocity: velocity,
                                ),
                              );
                              resetTooltipGesture();
                              return;
                            }

                            if (tooltipData.value != null && dragPointers.value.isEmpty && !isPinching.value) {
                              handleInteractionEnd();
                            }

                            resetTooltipGesture();
                          },
                          onPointerCancel: (event) {
                            dragPointers.value.remove(event.pointer);

                            if (tooltipGesturePointer.value != event.pointer) {
                              return;
                            }

                            if (isTooltipScrollGesture.value) {
                              resetTooltipGesture(cancelScrollDrag: true);
                              return;
                            }

                            if (tooltipData.value != null && dragPointers.value.isEmpty && !isPinching.value) {
                              handleInteractionEnd();
                            }

                            resetTooltipGesture();
                          },
                          child: GestureDetector(
                            behavior: HitTestBehavior.translucent,
                            onScaleStart: (_) {},
                            onScaleUpdate: (details) {
                              if (details.pointerCount >= 2) {
                                if (!isPinching.value) {
                                  beginPinch(details.localFocalPoint.dx);
                                }
                                updatePinch(details.scale);
                              }
                            },
                            onScaleEnd: (_) {
                              isPinching.value = false;
                              updateScrollState();
                            },
                            onLongPressStart: (details) {
                              if (isPinching.value || dragPointers.value.length != 1) {
                                return;
                              }

                              tooltipGestureStartLocalPosition.value = details.localPosition;
                              tooltipGestureLastTimeStamp.value = null;
                              handleInteraction(details.localPosition, withHaptic: true);
                            },
                            onTapDown: isTooltipVisible
                                ? (details) => handleInteraction(details.localPosition, withHaptic: false)
                                : null,
                            onTapUp: (details) {
                              handleInteraction(details.localPosition, withHaptic: false);
                              handleInteractionEnd();
                            },
                          ),
                        ),
                      ),
                      IgnorePointer(
                        child: SizedBox.expand(
                          child: Fader(
                            show: isTooltipShown.value,
                            duration: tooltipFadeDuration,
                            child: tooltipData.value == null
                                ? const SizedBox.shrink()
                                : CustomSingleChildLayout(
                                    delegate: _ChartTooltipPositionDelegate(
                                      chartSize: Size(chartWidth, chartHeight + xAxisAreaHeight),
                                      anchor: Offset(
                                        tooltipData.value!.index * barWidth + (barWidth / 2) - scrollOffset.value,
                                        0,
                                      ),
                                    ),
                                    child: Container(
                                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                                      decoration: BoxDecoration(
                                        color: context.colors.surfaceDark,
                                        borderRadius: BorderRadius.circular(6),
                                      ),
                                      child: IntrinsicWidth(
                                        child: Column(
                                          crossAxisAlignment: CrossAxisAlignment.start,
                                          mainAxisSize: MainAxisSize.min,
                                          children: [
                                            Text(
                                              tooltipData.value!.dayData.date.format(pattern: 'yyyy년 M월 d일'),
                                              style: TextStyle(
                                                color: context.colors.textBright,
                                                fontSize: 13,
                                                fontWeight: FontWeight.w600,
                                                decoration: TextDecoration.none,
                                              ),
                                            ),
                                            if (tooltipData.value!.dayData.additions > 0)
                                              Text(
                                                '입력: ${tooltipData.value!.dayData.additions.comma}자',
                                                style: TextStyle(
                                                  color: context.colors.textBright,
                                                  fontSize: 12,
                                                  decoration: TextDecoration.none,
                                                ),
                                              ),
                                            if (tooltipData.value!.dayData.deletions > 0)
                                              Text(
                                                '지움: ${tooltipData.value!.dayData.deletions.comma}자',
                                                style: TextStyle(
                                                  color: context.colors.textBright,
                                                  fontSize: 12,
                                                  decoration: TextDecoration.none,
                                                ),
                                              ),
                                            if (tooltipData.value!.dayData.total == 0)
                                              Text(
                                                '기록이 없어요',
                                                style: TextStyle(
                                                  color: context.colors.textBright,
                                                  fontSize: 12,
                                                  decoration: TextDecoration.none,
                                                ),
                                              ),
                                          ],
                                        ),
                                      ),
                                    ),
                                  ),
                          ),
                        ),
                      ),
                      Positioned(
                        left: 0,
                        top: 0,
                        bottom: 0,
                        child: IgnorePointer(
                          ignoring: !canScrollLeft.value,
                          child: Fader(
                            show: canScrollLeft.value,
                            duration: const Duration(milliseconds: 100),
                            child: Tappable(
                              onTap: () => scrollByViewport(-1),
                              child: Container(
                                padding: const EdgeInsets.symmetric(horizontal: 8),
                                decoration: BoxDecoration(
                                  gradient: LinearGradient(
                                    colors: [
                                      context.theme.scaffoldBackgroundColor.withValues(alpha: 0.8),
                                      context.theme.scaffoldBackgroundColor.withValues(alpha: 0),
                                    ],
                                    stops: const [0.3, 1],
                                  ),
                                ),
                                child: Center(
                                  child: Icon(Icons.chevron_left, size: 20, color: context.colors.textSubtle),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                      Positioned(
                        right: 0,
                        top: 0,
                        bottom: 0,
                        child: IgnorePointer(
                          ignoring: !canScrollRight.value,
                          child: Fader(
                            show: canScrollRight.value,
                            duration: const Duration(milliseconds: 100),
                            child: Tappable(
                              onTap: () => scrollByViewport(1),
                              child: Container(
                                padding: const EdgeInsets.symmetric(horizontal: 8),
                                decoration: BoxDecoration(
                                  gradient: LinearGradient(
                                    colors: [
                                      context.theme.scaffoldBackgroundColor.withValues(alpha: 0),
                                      context.theme.scaffoldBackgroundColor.withValues(alpha: 0.8),
                                    ],
                                    stops: const [0, 0.7],
                                  ),
                                ),
                                child: Center(
                                  child: Icon(Icons.chevron_right, size: 20, color: context.colors.textSubtle),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            );
          },
        ),
        const SizedBox(height: 8),
        Align(
          alignment: Alignment.centerRight,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              _LegendToggle(
                label: '입력한 글자',
                color: context.theme.brightness == Brightness.dark ? AppColors.dark.green_600 : AppColors.green_400,
                selected: showAdditions.value,
                onTap: () {
                  showAdditions.value = !showAdditions.value;
                },
              ),
              const SizedBox(width: 16),
              _LegendToggle(
                label: '지운 글자',
                color: context.theme.brightness == Brightness.dark ? AppColors.dark.gray_600 : AppColors.gray_400,
                selected: showDeletions.value,
                onTap: () {
                  showDeletions.value = !showDeletions.value;
                },
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _LegendToggle extends StatelessWidget {
  const _LegendToggle({required this.label, required this.color, required this.selected, required this.onTap});

  final String label;
  final Color color;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Opacity(
            opacity: selected ? 1 : 0.3,
            child: Container(
              width: 12,
              height: 12,
              decoration: BoxDecoration(color: color, borderRadius: BorderRadius.circular(2)),
            ),
          ),
          const SizedBox(width: 6),
          Text(
            label,
            style: TextStyle(fontSize: 13, color: selected ? context.colors.textSubtle : context.colors.textFaint),
          ),
        ],
      ),
    );
  }
}

class _DayData {
  const _DayData({required this.date, required this.additions, required this.deletions})
    : total = additions + deletions;

  final Jiffy date;
  final int additions;
  final int deletions;
  final int total;
}

class _BarHeights {
  const _BarHeights({required this.additionHeight, required this.deletionHeight, required this.effectiveTotal});

  final double additionHeight;
  final double deletionHeight;
  final int effectiveTotal;
}

class _VisibleRange {
  const _VisibleRange({required this.start, required this.end});

  final int start;
  final int end;
}

class _XAxisLabel {
  const _XAxisLabel({required this.index, required this.text, required this.isFirst, required this.isLast});

  final int index;
  final String text;
  final bool isFirst;
  final bool isLast;
}

class _PositionedXAxisLabel {
  const _PositionedXAxisLabel({required this.data, required this.left, required this.right});

  final _XAxisLabel data;
  final double left;
  final double right;
}

class _ChartTooltipPositionDelegate extends SingleChildLayoutDelegate {
  const _ChartTooltipPositionDelegate({required this.chartSize, required this.anchor});

  final Size chartSize;
  final Offset anchor;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    return BoxConstraints.loose(constraints.biggest);
  }

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    var dx = anchor.dx - (childSize.width / 2);
    dx = dx.clamp(8, chartSize.width - childSize.width - 8);

    var dy = -childSize.height - 4;
    final minDy = -childSize.height - 8;
    final maxDy = chartSize.height - childSize.height;
    if (dy < minDy) {
      dy = minDy;
    } else if (dy > maxDy) {
      dy = maxDy;
    }

    return Offset(dx, dy);
  }

  @override
  bool shouldRelayout(covariant _ChartTooltipPositionDelegate oldDelegate) {
    return chartSize != oldDelegate.chartSize || anchor != oldDelegate.anchor;
  }
}

List<_DayData> _generateDaysData(List<StatsCharacterCountChange> characterCountChanges) {
  final endDate = Jiffy.now();
  final startDate = endDate.subtract(days: 89);

  final changesByDate = <String, StatsCharacterCountChange>{
    for (final change in characterCountChanges) change.date.toLocal().format(pattern: 'yyyy-MM-dd'): change,
  };

  final data = <_DayData>[];

  var currentDate = startDate.clone();
  while (!currentDate.isAfter(endDate)) {
    final key = currentDate.format(pattern: 'yyyy-MM-dd');
    final change = changesByDate[key];

    final additions = change?.additions ?? 0;
    final deletions = (change?.deletions ?? 0).abs();

    data.add(_DayData(date: currentDate.clone(), additions: additions, deletions: deletions));

    currentDate = currentDate.add(days: 1);
  }

  return data;
}

_VisibleRange _calculateVisibleRange({
  required int length,
  required double barWidth,
  required double viewportWidth,
  required double scrollOffset,
}) {
  if (length == 0 || barWidth <= 0) {
    return const _VisibleRange(start: 0, end: 0);
  }

  final start = (scrollOffset / barWidth).floor().clamp(0, length - 1);
  final end = ((scrollOffset + viewportWidth) / barWidth).ceil().clamp(start + 1, length);

  return _VisibleRange(start: start, end: end);
}

int _maxValue(
  List<_DayData> daysData,
  bool showAdditions,
  bool showDeletions, {
  required int startIndex,
  required int endIndex,
}) {
  if (daysData.isEmpty || endIndex <= startIndex) {
    return 1000;
  }

  var maxVal = 0;

  for (var i = startIndex; i < endIndex; i++) {
    final day = daysData[i];
    final total = (showAdditions ? day.additions : 0) + (showDeletions ? day.deletions : 0);
    if (total > maxVal) {
      maxVal = total;
    }
  }

  return math.max(maxVal, 1000);
}

_BarHeights _calculateBarHeights({
  required _DayData dayData,
  required bool showAdditions,
  required bool showDeletions,
  required double chartHeight,
  required int maxVal,
}) {
  final effectiveAdditions = showAdditions ? dayData.additions : 0;
  final effectiveDeletions = showDeletions ? dayData.deletions : 0;
  final totalValue = effectiveAdditions + effectiveDeletions;

  final additionHeight = effectiveAdditions > 0 ? (effectiveAdditions / maxVal) * chartHeight : 0.0;
  final deletionHeight = effectiveDeletions > 0 ? (effectiveDeletions / maxVal) * chartHeight : 0.0;

  return _BarHeights(additionHeight: additionHeight, deletionHeight: deletionHeight, effectiveTotal: totalValue);
}

List<_XAxisLabel> _generateXAxisLabels(List<_DayData> daysData, double zoom) {
  final labels = <_XAxisLabel>[];
  final minGap = zoom >= 3
      ? 2
      : zoom >= 2.2
      ? 3
      : zoom >= 1.6
      ? 4
      : 5;
  final showWeekly = zoom >= 1.6;
  final showDense = zoom >= 2.6;
  var lastShownIndex = -999;

  for (var index = 0; index < daysData.length; index++) {
    final isFirstDay = index == 0;
    final isLastDay = index == daysData.length - 1;
    final isFirstOfMonth = daysData[index].date.date == 1;
    final isIntervalLabel = showDense ? index % 3 == 0 : showWeekly && index % 7 == 0;
    final shouldShowLabel = isFirstDay || isLastDay || isFirstOfMonth || isIntervalLabel;

    if (!shouldShowLabel) {
      continue;
    }

    if (!isFirstDay && !isLastDay && index - lastShownIndex < minGap) {
      continue;
    }

    labels.add(
      _XAxisLabel(
        index: index,
        text: daysData[index].date.format(pattern: 'M/d'),
        isFirst: isFirstDay,
        isLast: isLastDay,
      ),
    );
    lastShownIndex = index;
  }

  final lastIndex = daysData.length - 1;
  if (lastIndex >= 0 && labels.isNotEmpty && labels.last.index != lastIndex) {
    if (lastIndex - labels.last.index < minGap && labels.length > 1) {
      labels.removeLast();
    }

    labels.add(
      _XAxisLabel(
        index: lastIndex,
        text: daysData[lastIndex].date.format(pattern: 'M/d'),
        isFirst: false,
        isLast: true,
      ),
    );
  }

  return labels;
}

List<_PositionedXAxisLabel> _positionXAxisLabels({
  required List<_XAxisLabel> labels,
  required double barWidth,
  required double chartWidth,
  required TextStyle textStyle,
  required TextDirection textDirection,
}) {
  if (labels.isEmpty || barWidth <= 0 || chartWidth <= 0) {
    return const [];
  }

  const minGapPx = 8.0;
  final positioned = <_PositionedXAxisLabel>[];

  for (final label in labels) {
    final painter = TextPainter(
      text: TextSpan(text: label.text, style: textStyle),
      textDirection: textDirection,
      maxLines: 1,
    )..layout();

    final width = painter.width;
    if (width <= 0) {
      continue;
    }

    var left = label.index * barWidth - (width / 2);
    final maxLeft = (chartWidth - width) < 0 ? 0.0 : (chartWidth - width);
    left = left < 0 ? 0.0 : (left > maxLeft ? maxLeft : left);
    var right = left + width;

    if (positioned.isNotEmpty && left < positioned.last.right + minGapPx) {
      if (!label.isLast) {
        continue;
      }

      while (positioned.isNotEmpty && left < positioned.last.right + minGapPx && !positioned.last.data.isFirst) {
        positioned.removeLast();
      }

      if (positioned.isNotEmpty && left < positioned.last.right + minGapPx) {
        final candidateLeft = positioned.last.right + minGapPx;
        left = candidateLeft < 0 ? 0.0 : (candidateLeft > maxLeft ? maxLeft : candidateLeft);
        right = left + width;
      }
    }

    positioned.add(_PositionedXAxisLabel(data: label, left: left, right: right));
  }

  return positioned;
}
