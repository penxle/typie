import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/screens/profile/__generated__/profile_query.data.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

class ActivityGrid extends HookWidget {
  const ActivityGrid({super.key, required this.characterCountChanges});

  final List<GProfileScreen_QueryData_me_characterCountChanges> characterCountChanges;

  static const cellSize = 13.0;
  static const cellGap = 3.0;
  static const labelHeight = 16.0;

  @override
  Widget build(BuildContext context) {
    final scrollController = useScrollController();
    final canScrollLeft = useState(false);
    final canScrollRight = useState(true);
    final tooltipData = useState<({Activity activity, int weekIndex, int dayIndex})?>(null);
    final tooltipTimer = useRef<Timer?>(null);

    useAsyncEffect(() async {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (scrollController.hasClients) {
          scrollController.jumpTo(scrollController.position.maxScrollExtent);
          _updateScrollState(scrollController, canScrollLeft, canScrollRight);
        }
      });

      void updateScroll() {
        _updateScrollState(scrollController, canScrollLeft, canScrollRight);
      }

      scrollController.addListener(updateScroll);

      return () => scrollController.removeListener(updateScroll);
    }, []);

    final endDate = Jiffy.now();
    final startDate = endDate.subtract(days: 364);

    final activities = useMemoized(() => _generateActivities(startDate, endDate), [characterCountChanges]);
    final monthSpans = useMemoized(() => _generateMonthSpans(startDate, endDate), [characterCountChanges]);

    final weekCount = ((endDate.diff(startDate, unit: Unit.day) + 1) / 7).ceil();
    final totalWidth = weekCount * (cellSize + cellGap) - cellGap;

    void scrollByMonths(int monthDelta) {
      if (!scrollController.hasClients) {
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

    void scrollLeft() => scrollByMonths(-2);
    void scrollRight() => scrollByMonths(2);

    return Stack(
      clipBehavior: Clip.none,
      children: [
        SingleChildScrollView(
          controller: scrollController,
          scrollDirection: Axis.horizontal,
          physics: const NeverScrollableScrollPhysics(),
          padding: const Pad(horizontal: 16, bottom: 8),
          child: SizedBox(
            width: totalWidth,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onHorizontalDragUpdate: (details) {
                    scrollController.jumpTo(
                      (scrollController.offset - details.delta.dx).clamp(
                        0.0,
                        scrollController.position.maxScrollExtent,
                      ),
                    );
                  },
                  child: SizedBox(
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
                ),
                const SizedBox(height: cellGap),
                _buildActivityGrid(context, activities, scrollController, tooltipData, tooltipTimer),
              ],
            ),
          ),
        ),
        Positioned(
          left: 0,
          top: 0,
          bottom: 0,
          child: AnimatedOpacity(
            opacity: canScrollLeft.value ? 1.0 : 0.0,
            duration: const Duration(milliseconds: 100),
            child: IgnorePointer(
              ignoring: !canScrollLeft.value,
              child: Tappable(
                onTap: scrollLeft,
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
          child: AnimatedOpacity(
            opacity: canScrollRight.value ? 1.0 : 0.0,
            duration: const Duration(milliseconds: 100),
            child: IgnorePointer(
              ignoring: !canScrollRight.value,
              child: Tappable(
                onTap: scrollRight,
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
        if (tooltipData.value != null)
          Positioned.fill(
            child: CustomSingleChildLayout(
              delegate: _TooltipPositionDelegate(
                cellPosition: Offset(
                  tooltipData.value!.weekIndex * (cellSize + cellGap) + 16 - scrollController.offset,
                  tooltipData.value!.dayIndex * (cellSize + cellGap) + labelHeight + cellGap,
                ),
                cellSize: const Size(cellSize, cellSize),
              ),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: BoxDecoration(color: context.colors.surfaceDark, borderRadius: BorderRadius.circular(6)),
                child: IntrinsicWidth(
                  child: IntrinsicHeight(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          tooltipData.value!.activity.date.format(pattern: 'yyyy년 M월 d일'),
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
      ],
    );
  }

  void _updateScrollState(
    ScrollController controller,
    ValueNotifier<bool> canScrollLeft,
    ValueNotifier<bool> canScrollRight,
  ) {
    if (controller.hasClients) {
      canScrollLeft.value = controller.offset > 0;
      canScrollRight.value = controller.offset < controller.position.maxScrollExtent;
    }
  }

  Widget _buildActivityGrid(
    BuildContext context,
    List<Activity> activities,
    ScrollController scrollController,
    ValueNotifier<({Activity activity, int weekIndex, int dayIndex})?> tooltipData,
    ObjectRef<Timer?> tooltipTimer,
  ) {
    final selectedCell = useState<({int weekIndex, int dayIndex})?>(null);
    final weeks = <List<Activity>>[];
    final weekCount = (activities.length / 7).ceil();

    for (var week = 0; week < weekCount; week++) {
      final weekActivities = <Activity>[];
      for (var day = 0; day < 7; day++) {
        final index = week * 7 + day;
        if (index < activities.length) {
          weekActivities.add(activities[index]);
        }
      }
      weeks.add(weekActivities);
    }

    ({Activity? activity, int weekIndex, int dayIndex})? getActivityAtPosition(Offset localPosition) {
      final weekIndex = (localPosition.dx / (cellSize + cellGap)).floor();
      final dayIndex = (localPosition.dy / (cellSize + cellGap)).floor();

      if (weekIndex >= 0 &&
          weekIndex < weeks.length &&
          dayIndex >= 0 &&
          dayIndex < 7 &&
          dayIndex < weeks[weekIndex].length) {
        final activity = weeks[weekIndex][dayIndex];
        if (activity.level != -1) {
          return (activity: activity, weekIndex: weekIndex, dayIndex: dayIndex);
        }
      }
      return null;
    }

    void showActivityTooltip(Activity? activity, int weekIndex, int dayIndex) {
      if (activity != null) {
        tooltipTimer.value?.cancel();
        selectedCell.value = (weekIndex: weekIndex, dayIndex: dayIndex);
        tooltipData.value = (activity: activity, weekIndex: weekIndex, dayIndex: dayIndex);
      }
    }

    void hideAfterDelay(Duration delay) {
      tooltipTimer.value?.cancel();
      tooltipTimer.value = Timer(delay, () {
        tooltipData.value = null;
        selectedCell.value = null;
      });
    }

    void handleTooltipInteraction(Offset localPosition) {
      final result = getActivityAtPosition(localPosition);
      if (result != null) {
        showActivityTooltip(result.activity, result.weekIndex, result.dayIndex);
      }
    }

    void handleInteractionEnd() {
      hideAfterDelay(const Duration(seconds: 1));
    }

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onVerticalDragStart: (details) => handleTooltipInteraction(details.localPosition),
      onVerticalDragUpdate: (details) => handleTooltipInteraction(details.localPosition),
      onVerticalDragEnd: (_) => handleInteractionEnd(),
      onPanStart: (details) => handleTooltipInteraction(details.localPosition),
      onPanUpdate: (details) => handleTooltipInteraction(details.localPosition),
      onPanEnd: (_) => handleInteractionEnd(),
      onTapDown: (details) => handleTooltipInteraction(details.localPosition),
      onTapUp: (_) => handleInteractionEnd(),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (int week = 0; week < weeks.length; week++)
            Padding(
              padding: EdgeInsets.only(right: week < weeks.length - 1 ? cellGap : 0),
              child: Column(
                children: [
                  for (int day = 0; day < 7; day++)
                    if (day < weeks[week].length)
                      Padding(
                        padding: EdgeInsets.only(bottom: day < 6 ? cellGap : 0),
                        child: weeks[week][day].level == -1
                            ? const SizedBox(width: cellSize, height: cellSize)
                            : Stack(
                                clipBehavior: Clip.none,
                                children: [
                                  Container(
                                    width: cellSize,
                                    height: cellSize,
                                    decoration: BoxDecoration(
                                      color: _getColorByLevel(context, weeks[week][day].level),
                                      borderRadius: BorderRadius.circular(2),
                                    ),
                                  ),
                                  if (selectedCell.value != null &&
                                      selectedCell.value!.weekIndex == week &&
                                      selectedCell.value!.dayIndex == day)
                                    Positioned(
                                      left: -1.5,
                                      top: -1.5,
                                      child: Container(
                                        width: cellSize + 3,
                                        height: cellSize + 3,
                                        decoration: BoxDecoration(
                                          border: Border.all(color: context.colors.borderStrong, width: 1.5),
                                          borderRadius: BorderRadius.circular(3.5),
                                        ),
                                      ),
                                    ),
                                ],
                              ),
                      ),
                ],
              ),
            ),
        ],
      ),
    );
  }

  List<Activity> _generateActivities(Jiffy startDate, Jiffy endDate) {
    final activities = <Activity>[];
    final changesMap = <String, GProfileScreen_QueryData_me_characterCountChanges>{};

    for (final change in characterCountChanges) {
      final date = change.date.toLocal();
      final dateKey = date.format(pattern: 'yyyy-MM-dd');
      changesMap[dateKey] = change;
    }

    final numbers = characterCountChanges.map((c) => c.additions).where((n) => n > 0).toList();

    var p95 = 0;
    if (numbers.isNotEmpty) {
      final sorted = List<int>.from(numbers)..sort();
      final index = (sorted.length * 0.95).floor();
      p95 = sorted[index.clamp(0, sorted.length - 1)];
    }

    for (var i = 1; i < startDate.dayOfWeek; i++) {
      activities.add(Activity(date: startDate.subtract(days: i), additions: -1, level: -1));
    }

    var currentDate = startDate.clone();
    while (!currentDate.isAfter(endDate)) {
      final key = currentDate.format(pattern: 'yyyy-MM-dd');
      final change = changesMap[key];

      int level;
      int additions;

      if (change != null) {
        additions = change.additions;
        if (additions == 0) {
          level = 0;
        } else if (p95 == 0) {
          level = 3;
        } else if (additions >= p95) {
          level = 5;
        } else {
          final ratio = additions / p95;
          level = (ratio * 4).floor() + 1;
          if (level > 4) {
            level = 4;
          }
        }
      } else {
        additions = 0;
        level = 0;
      }

      activities.add(Activity(date: currentDate.clone(), additions: additions, level: level));

      currentDate = currentDate.add(days: 1);
    }

    return activities;
  }

  List<MonthSpan> _generateMonthSpans(Jiffy startDate, Jiffy endDate) {
    final monthSpans = <MonthSpan>[];

    final activities = _generateActivities(startDate, endDate);
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

        if (activity.date.date == 1) {
          hasFirstOfMonth = true;
          weekMonth = activity.date.month;
          break;
        }
      }

      if (weekIndex == 0 || (hasFirstOfMonth && weekMonth != prevMonth)) {
        if (monthStartWeek >= 0 && prevMonth != -1) {
          monthSpans.add(MonthSpan(month: prevMonth, start: monthStartWeek, end: weekIndex - 1));
        }

        monthStartWeek = weekIndex;
        prevMonth = weekMonth;
      }
    }

    if (monthStartWeek >= 0 && prevMonth != -1) {
      monthSpans.add(MonthSpan(month: prevMonth, start: monthStartWeek, end: weekCount - 1));
    }

    return monthSpans;
  }

  Color _getColorByLevel(BuildContext context, int level) {
    final isDark = context.theme.brightness == Brightness.dark;

    switch (level) {
      case 0:
        return isDark ? AppColors.dark.gray_800 : AppColors.gray_100;
      case 1:
        return isDark ? AppColors.dark.green_800 : AppColors.green_100;
      case 2:
        return isDark ? AppColors.dark.green_600 : AppColors.green_300;
      case 3:
        return isDark ? AppColors.dark.green_500 : AppColors.green_500;
      case 4:
        return isDark ? AppColors.dark.green_300 : AppColors.green_700;
      case 5:
        return isDark ? AppColors.dark.green_100 : AppColors.green_900;
      default:
        return isDark ? AppColors.dark.gray_800 : AppColors.gray_100;
    }
  }
}

class Activity {
  Activity({required this.date, required this.additions, required this.level});

  final Jiffy date;
  final int additions;
  final int level;
}

class MonthSpan {
  MonthSpan({required this.month, required this.start, required this.end});

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
