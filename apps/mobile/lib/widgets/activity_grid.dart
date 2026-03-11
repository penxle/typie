import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/fader.dart';
import 'package:typie/widgets/tappable.dart';

class ActivityGridChange {
  const ActivityGridChange({required this.date, required this.additions});

  final Jiffy date;
  final int additions;
}

class ActivityGrid extends HookWidget {
  const ActivityGrid({super.key, required this.changes});

  final List<ActivityGridChange> changes;

  static const cellSize = 13.0;
  static const cellGap = 3.0;
  static const gridHeight = (cellSize * 7) + (cellGap * 6);
  static const labelHeight = 16.0;
  static const horizontalPadding = 16.0;
  static const bottomPadding = 8.0;
  static const tooltipScrollActivationDistance = 12.0;
  static const tooltipScrollVelocityThreshold = 700.0;

  @override
  Widget build(BuildContext context) {
    final endDate = _startOfDay(DateTime.now());
    final startDate = endDate.subtract(const Duration(days: 364));

    final activities = useMemoized(() => _generateActivities(changes, startDate, endDate), [changes]);
    final monthSpans = useMemoized(() => _generateMonthSpans(activities), [activities]);
    final weeks = useMemoized(() => _generateWeeks(activities), [activities]);
    final totalWidth = weeks.isEmpty ? 0.0 : (weeks.length * (cellSize + cellGap)) - cellGap;

    final scrollController = useScrollController(initialScrollOffset: totalWidth, keys: [totalWidth]);
    final canScrollLeft = useState(false);
    final canScrollRight = useState(false);
    final tooltipData = useState<({ActivityGridActivity activity, int weekIndex, int dayIndex})?>(null);
    final tooltipTimer = useRef<Timer?>(null);

    final dragPointers = useRef(<int>{});
    final tooltipGesturePointer = useRef<int?>(null);
    final tooltipGestureLastTimeStamp = useRef<Duration?>(null);
    final tooltipGestureStartLocalPosition = useRef<Offset?>(null);
    final tooltipGestureVelocityTracker = useRef<VelocityTracker?>(null);
    final tooltipScrollDrag = useRef<Drag?>(null);
    final isTooltipScrollGesture = useRef(false);

    void updateScrollState() {
      if (!scrollController.hasClients) {
        canScrollLeft.value = false;
        canScrollRight.value = false;
        return;
      }

      canScrollLeft.value = scrollController.offset > 0;
      canScrollRight.value = scrollController.offset < scrollController.position.maxScrollExtent;
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
      WidgetsBinding.instance.addPostFrameCallback((_) {
        updateScrollState();
      });

      void updateScroll() {
        updateScrollState();
      }

      scrollController.addListener(updateScroll);

      return () => scrollController.removeListener(updateScroll);
    }, [scrollController]);

    useEffect(() {
      return () {
        tooltipScrollDrag.value?.cancel();
        tooltipTimer.value?.cancel();
      };
    }, []);

    final isTooltipVisible = tooltipData.value != null;
    final isDark = context.theme.brightness == Brightness.dark;
    final levelColors = useMemoized(() => _levelColors(isDark), [isDark]);

    void showTooltip(ActivityGridActivity? activity, int weekIndex, int dayIndex, {required bool withHaptic}) {
      if (activity == null) {
        return;
      }

      tooltipTimer.value?.cancel();

      final prevTooltipData = tooltipData.value;
      final isCellChanged =
          prevTooltipData == null || prevTooltipData.weekIndex != weekIndex || prevTooltipData.dayIndex != dayIndex;

      if (withHaptic && isCellChanged) {
        unawaited(HapticFeedback.selectionClick());
      }

      tooltipData.value = (activity: activity, weekIndex: weekIndex, dayIndex: dayIndex);
    }

    void hideAfterDelay(Duration delay) {
      tooltipTimer.value?.cancel();
      tooltipTimer.value = Timer(delay, () {
        tooltipData.value = null;
      });
    }

    ({ActivityGridActivity? activity, int weekIndex, int dayIndex})? getActivityAtPosition(Offset localPosition) {
      final weekIndex = (localPosition.dx / (cellSize + cellGap)).floor();
      final dayIndex = (localPosition.dy / (cellSize + cellGap)).floor();

      if (weekIndex < 0 ||
          weekIndex >= weeks.length ||
          dayIndex < 0 ||
          dayIndex >= 7 ||
          dayIndex >= weeks[weekIndex].length) {
        return null;
      }

      final activity = weeks[weekIndex][dayIndex];
      if (activity.level == -1) {
        return null;
      }

      return (activity: activity, weekIndex: weekIndex, dayIndex: dayIndex);
    }

    void handleTooltipInteraction(Offset localPosition, {required bool withHaptic}) {
      final result = getActivityAtPosition(localPosition);
      if (result != null) {
        showTooltip(result.activity, result.weekIndex, result.dayIndex, withHaptic: withHaptic);
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

      tooltipTimer.value?.cancel();
      tooltipData.value = null;
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

    void scrollByMonths(int monthDelta) {
      if (!scrollController.hasClients || monthSpans.isEmpty) {
        return;
      }

      final currentPosition = scrollController.offset;
      final currentWeekIndex = (currentPosition / (cellSize + cellGap)).floor();

      var currentMonthIndex = 0;
      for (var i = 0; i < monthSpans.length; i++) {
        if (monthSpans[i].start <= currentWeekIndex && monthSpans[i].end >= currentWeekIndex) {
          currentMonthIndex = i;
          break;
        }
        if (monthSpans[i].start > currentWeekIndex) {
          currentMonthIndex = i - 1;
          break;
        }
      }

      final targetMonthIndex = (currentMonthIndex + monthDelta).clamp(0, monthSpans.length - 1);
      final targetWeekIndex = monthSpans[targetMonthIndex].start;
      final targetPosition = targetWeekIndex * (cellSize + cellGap);

      unawaited(
        scrollController.animateTo(
          targetPosition.clamp(0.0, scrollController.position.maxScrollExtent),
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOutCubic,
        ),
      );
    }

    return Stack(
      clipBehavior: Clip.none,
      children: [
        SingleChildScrollView(
          controller: scrollController,
          scrollDirection: Axis.horizontal,
          physics: isTooltipVisible ? const NeverScrollableScrollPhysics() : null,
          padding: const Pad(horizontal: horizontalPadding, bottom: bottomPadding),
          child: SizedBox(
            width: totalWidth,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SizedBox(
                  height: labelHeight + 12,
                  child: Stack(
                    clipBehavior: Clip.none,
                    alignment: Alignment.bottomCenter,
                    children: [
                      for (int i = 0; i < monthSpans.length; i++)
                        if (monthSpans[i].end - monthSpans[i].start >= 1 || i == monthSpans.length - 1)
                          Positioned(
                            left: monthSpans[i].start * (cellSize + cellGap),
                            child: GestureDetector(
                              onTap: () {
                                final span = monthSpans[i];
                                final monthWidth = (span.end - span.start + 1) * (cellSize + cellGap) - cellGap;
                                final monthStartPosition = span.start * (cellSize + cellGap);
                                final monthCenterPosition = monthStartPosition + (monthWidth / 2);
                                final viewportWidth = scrollController.position.viewportDimension;
                                final targetPosition = monthCenterPosition - (viewportWidth / 2);

                                unawaited(
                                  scrollController.animateTo(
                                    targetPosition.clamp(0.0, scrollController.position.maxScrollExtent),
                                    duration: const Duration(milliseconds: 200),
                                    curve: Curves.easeOutCubic,
                                  ),
                                );
                              },
                              child: SizedBox(
                                width: (monthSpans[i].end - monthSpans[i].start + 1) * (cellSize + cellGap) - cellGap,
                                child: Text(
                                  '${monthSpans[i].month}월',
                                  overflow: TextOverflow.visible,
                                  softWrap: false,
                                  style: TextStyle(
                                    fontSize: 11,
                                    fontWeight: FontWeight.w500,
                                    color: context.colors.textFaint,
                                  ),
                                ),
                              ),
                            ),
                          ),
                    ],
                  ),
                ),
                const SizedBox(height: cellGap),
                Listener(
                  behavior: HitTestBehavior.opaque,
                  onPointerDown: (event) {
                    dragPointers.value.add(event.pointer);

                    if (dragPointers.value.length > 1) {
                      resetTooltipGesture(cancelScrollDrag: true);
                      return;
                    }

                    beginTooltipGestureTracking(event);

                    if (tooltipData.value != null) {
                      handleTooltipInteraction(event.localPosition, withHaptic: false);
                    }
                  },
                  onPointerMove: (event) {
                    if (!dragPointers.value.contains(event.pointer) || tooltipGesturePointer.value != event.pointer) {
                      return;
                    }

                    tooltipGestureVelocityTracker.value?.addPosition(event.timeStamp, event.position);

                    if (dragPointers.value.length != 1) {
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
                        final velocity = event.delta.dx.abs() * Duration.microsecondsPerSecond / elapsedMicroseconds;
                        final totalDistance = (event.localPosition.dx - startLocalPosition.dx).abs();

                        if (totalDistance >= tooltipScrollActivationDistance &&
                            velocity >= tooltipScrollVelocityThreshold) {
                          startTooltipScroll(event);
                          tooltipGestureLastTimeStamp.value = event.timeStamp;
                          return;
                        }
                      }
                    }

                    handleTooltipInteraction(event.localPosition, withHaptic: true);
                    tooltipGestureLastTimeStamp.value = event.timeStamp;
                  },
                  onPointerUp: (event) {
                    dragPointers.value.remove(event.pointer);

                    if (tooltipGesturePointer.value != event.pointer) {
                      return;
                    }

                    tooltipGestureVelocityTracker.value?.addPosition(event.timeStamp, event.position);

                    if (isTooltipScrollGesture.value) {
                      final velocity = tooltipGestureVelocityTracker.value?.getVelocity().pixelsPerSecond.dx ?? 0.0;
                      tooltipScrollDrag.value?.end(
                        DragEndDetails(
                          velocity: Velocity(pixelsPerSecond: Offset(velocity, 0)),
                          primaryVelocity: velocity,
                        ),
                      );
                      resetTooltipGesture();
                      return;
                    }

                    if (tooltipData.value != null && dragPointers.value.isEmpty) {
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

                    if (tooltipData.value != null && dragPointers.value.isEmpty) {
                      handleInteractionEnd();
                    }

                    resetTooltipGesture();
                  },
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onLongPressStart: (details) {
                      if (dragPointers.value.length != 1) {
                        return;
                      }

                      tooltipGestureStartLocalPosition.value = details.localPosition;
                      tooltipGestureLastTimeStamp.value = null;
                      handleTooltipInteraction(details.localPosition, withHaptic: true);
                    },
                    onTapDown: isTooltipVisible
                        ? (details) => handleTooltipInteraction(details.localPosition, withHaptic: false)
                        : null,
                    onTapUp: (details) {
                      handleTooltipInteraction(details.localPosition, withHaptic: false);
                      handleInteractionEnd();
                    },
                    child: SizedBox(
                      width: totalWidth,
                      height: gridHeight,
                      child: CustomPaint(
                        painter: _ActivityGridPainter(
                          weeks: weeks,
                          levelColors: levelColors,
                          selectedWeekIndex: tooltipData.value?.weekIndex,
                          selectedDayIndex: tooltipData.value?.dayIndex,
                          selectionBorderColor: context.colors.surfaceDark,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
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
                onTap: () => scrollByMonths(-2),
                child: Container(
                  padding: const Pad(horizontal: 8),
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      colors: [
                        context.theme.scaffoldBackgroundColor.withValues(alpha: 0.8),
                        context.theme.scaffoldBackgroundColor.withValues(alpha: 0),
                      ],
                      stops: const [0.3, 1],
                    ),
                  ),
                  child: Center(child: Icon(Icons.chevron_left, size: 20, color: context.colors.textSubtle)),
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
                onTap: () => scrollByMonths(2),
                child: Container(
                  padding: const Pad(horizontal: 8),
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      colors: [
                        context.theme.scaffoldBackgroundColor.withValues(alpha: 0),
                        context.theme.scaffoldBackgroundColor.withValues(alpha: 0.8),
                      ],
                      stops: const [0, 0.7],
                    ),
                  ),
                  child: Center(child: Icon(Icons.chevron_right, size: 20, color: context.colors.textSubtle)),
                ),
              ),
            ),
          ),
        ),
        Positioned.fill(
          child: IgnorePointer(
            child: Fader(
              show: tooltipData.value != null,
              duration: const Duration(milliseconds: 140),
              child: tooltipData.value == null
                  ? const SizedBox.shrink()
                  : CustomSingleChildLayout(
                      delegate: _TooltipPositionDelegate(
                        cellPosition: Offset(
                          tooltipData.value!.weekIndex * (cellSize + cellGap) +
                              horizontalPadding -
                              scrollController.offset,
                          tooltipData.value!.dayIndex * (cellSize + cellGap) + labelHeight + cellGap,
                        ),
                        cellSize: const Size(cellSize, cellSize),
                      ),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                        decoration: BoxDecoration(
                          color: context.colors.surfaceDark,
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: IntrinsicWidth(
                          child: IntrinsicHeight(
                            child: Column(
                              mainAxisSize: MainAxisSize.min,
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  _formatDate(tooltipData.value!.activity.date),
                                  style: TextStyle(
                                    color: context.colors.textBright,
                                    fontSize: 12,
                                    fontWeight: FontWeight.w500,
                                    decoration: TextDecoration.none,
                                  ),
                                ),
                                Text(
                                  tooltipData.value!.activity.additions > 0
                                      ? '${tooltipData.value!.activity.additions.comma}자 작성했어요'
                                      : '기록이 없어요',
                                  style: TextStyle(
                                    color: context.colors.textBright,
                                    fontSize: 12,
                                    fontWeight: FontWeight.bold,
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
        ),
      ],
    );
  }

  List<ActivityGridActivity> _generateActivities(
    List<ActivityGridChange> characterCountChanges,
    DateTime startDate,
    DateTime endDate,
  ) {
    final activities = <ActivityGridActivity>[];
    final changesMap = <int, int>{};

    for (final change in characterCountChanges) {
      final date = _toLocalDate(change.date);
      changesMap[_dateKey(date)] = change.additions;
    }

    final numbers = characterCountChanges.map((c) => c.additions).where((n) => n > 0).toList();

    var p95 = 0;
    if (numbers.isNotEmpty) {
      final sorted = List<int>.from(numbers)..sort();
      final index = (sorted.length * 0.95).floor();
      p95 = sorted[index.clamp(0, sorted.length - 1)];
    }

    for (var i = 1; i <= startDate.weekday % 7; i++) {
      activities.add(ActivityGridActivity(date: startDate.subtract(Duration(days: i)), additions: -1, level: -1));
    }

    var currentDate = startDate;
    while (!currentDate.isAfter(endDate)) {
      final additionsFromChange = changesMap[_dateKey(currentDate)];

      late final int additions;
      late final int level;

      if (additionsFromChange != null) {
        additions = additionsFromChange;
        if (additions == 0) {
          level = 0;
        } else if (p95 == 0) {
          level = 3;
        } else if (additions >= p95) {
          level = 5;
        } else {
          final ratio = additions / p95;
          final computedLevel = (ratio * 4).floor() + 1;
          level = computedLevel > 4 ? 4 : computedLevel;
        }
      } else {
        additions = 0;
        level = 0;
      }

      activities.add(ActivityGridActivity(date: currentDate, additions: additions, level: level));
      currentDate = currentDate.add(const Duration(days: 1));
    }

    return activities;
  }

  List<List<ActivityGridActivity>> _generateWeeks(List<ActivityGridActivity> activities) {
    final weeks = <List<ActivityGridActivity>>[];
    final weekCount = (activities.length / 7).ceil();

    for (var week = 0; week < weekCount; week++) {
      final weekActivities = <ActivityGridActivity>[];
      for (var day = 0; day < 7; day++) {
        final index = week * 7 + day;
        if (index < activities.length) {
          weekActivities.add(activities[index]);
        }
      }
      weeks.add(weekActivities);
    }

    return weeks;
  }

  List<ActivityGridMonthSpan> _generateMonthSpans(List<ActivityGridActivity> activities) {
    final monthSpans = <ActivityGridMonthSpan>[];
    final weekCount = (activities.length / 7).ceil();

    var prevMonth = -1;
    var monthStartWeek = -1;

    for (var weekIndex = 0; weekIndex < weekCount; weekIndex++) {
      var weekMonth = -1;
      var hasFirstOfMonth = false;

      for (var dayIndex = 0; dayIndex < 7; dayIndex++) {
        final activityIndex = weekIndex * 7 + dayIndex;
        if (activityIndex >= activities.length) {
          break;
        }

        final activity = activities[activityIndex];
        if (activity.level == -1) {
          continue;
        }

        if (weekMonth == -1) {
          weekMonth = activity.date.month;
        }

        if (activity.date.day == 1) {
          hasFirstOfMonth = true;
          weekMonth = activity.date.month;
          break;
        }
      }

      if (weekIndex == 0 || (hasFirstOfMonth && weekMonth != prevMonth)) {
        if (monthStartWeek >= 0 && prevMonth != -1) {
          monthSpans.add(ActivityGridMonthSpan(month: prevMonth, start: monthStartWeek, end: weekIndex - 1));
        }

        monthStartWeek = weekIndex;
        prevMonth = weekMonth;
      }
    }

    if (monthStartWeek >= 0 && prevMonth != -1) {
      monthSpans.add(ActivityGridMonthSpan(month: prevMonth, start: monthStartWeek, end: weekCount - 1));
    }

    return monthSpans;
  }

  List<Color> _levelColors(bool isDark) {
    if (isDark) {
      return [
        AppColors.dark.gray_800,
        AppColors.dark.green_700,
        AppColors.dark.green_500,
        AppColors.dark.green_400,
        AppColors.dark.green_300,
        AppColors.dark.green_200,
      ];
    }

    return [
      AppColors.gray_200,
      AppColors.green_300,
      AppColors.green_500,
      AppColors.green_600,
      AppColors.green_700,
      AppColors.green_800,
    ];
  }

  DateTime _startOfDay(DateTime date) => DateTime(date.year, date.month, date.day);

  DateTime _toLocalDate(Jiffy date) {
    final local = date.toLocal();
    return DateTime(local.year, local.month, local.date);
  }

  int _dateKey(DateTime date) => (date.year * 10000) + (date.month * 100) + date.day;

  String _formatDate(DateTime date) => '${date.year}년 ${date.month}월 ${date.day}일';
}

class ActivityGridActivity {
  ActivityGridActivity({required this.date, required this.additions, required this.level});

  final DateTime date;
  final int additions;
  final int level;
}

class ActivityGridMonthSpan {
  ActivityGridMonthSpan({required this.month, required this.start, required this.end});

  final int month;
  final int start;
  int end;
}

class _TooltipPositionDelegate extends SingleChildLayoutDelegate {
  const _TooltipPositionDelegate({required this.cellPosition, required this.cellSize});

  final Offset cellPosition;
  final Size cellSize;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    return BoxConstraints.loose(constraints.biggest);
  }

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    const minX = 8.0;
    var tooltipX = cellPosition.dx - childSize.width - 2;
    final tooltipY = cellPosition.dy - childSize.height + cellSize.height - 2;

    if (tooltipX < minX) {
      tooltipX = minX;
    }

    return Offset(tooltipX, tooltipY);
  }

  @override
  bool shouldRelayout(covariant _TooltipPositionDelegate oldDelegate) {
    return cellPosition != oldDelegate.cellPosition || cellSize != oldDelegate.cellSize;
  }
}

class _ActivityGridPainter extends CustomPainter {
  const _ActivityGridPainter({
    required this.weeks,
    required this.levelColors,
    required this.selectionBorderColor,
    this.selectedWeekIndex,
    this.selectedDayIndex,
  });

  final List<List<ActivityGridActivity>> weeks;
  final List<Color> levelColors;
  final Color selectionBorderColor;
  final int? selectedWeekIndex;
  final int? selectedDayIndex;

  @override
  void paint(Canvas canvas, Size size) {
    final cellPaint = Paint()..style = PaintingStyle.fill;
    final selectionPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5
      ..color = selectionBorderColor;

    for (var week = 0; week < weeks.length; week++) {
      for (var day = 0; day < weeks[week].length; day++) {
        final activity = weeks[week][day];
        if (activity.level == -1) {
          continue;
        }

        final x = week * (ActivityGrid.cellSize + ActivityGrid.cellGap);
        final y = day * (ActivityGrid.cellSize + ActivityGrid.cellGap);
        final cellRect = Rect.fromLTWH(x, y, ActivityGrid.cellSize, ActivityGrid.cellSize);

        cellPaint.color = levelColors[activity.level.clamp(0, levelColors.length - 1)];
        canvas.drawRRect(RRect.fromRectAndRadius(cellRect, const Radius.circular(2)), cellPaint);

        if (selectedWeekIndex == week && selectedDayIndex == day) {
          final selectionRect = Rect.fromLTWH(x - 1.5, y - 1.5, ActivityGrid.cellSize + 3, ActivityGrid.cellSize + 3);
          canvas.drawRRect(RRect.fromRectAndRadius(selectionRect, const Radius.circular(3.5)), selectionPaint);
        }
      }
    }
  }

  @override
  bool shouldRepaint(covariant _ActivityGridPainter oldDelegate) {
    return weeks != oldDelegate.weeks ||
        levelColors != oldDelegate.levelColors ||
        selectionBorderColor != oldDelegate.selectionBorderColor ||
        selectedWeekIndex != oldDelegate.selectedWeekIndex ||
        selectedDayIndex != oldDelegate.selectedDayIndex;
  }
}
