import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
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
    final lastToastedActivity = useRef<Activity?>(null);

    useAsyncEffect(() async {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (scrollController.hasClients) {
          scrollController.jumpTo(scrollController.position.maxScrollExtent);
          _updateScrollState(scrollController, canScrollLeft, canScrollRight);
        }
      });

      void listener() {
        _updateScrollState(scrollController, canScrollLeft, canScrollRight);
      }

      scrollController.addListener(listener);

      return () => scrollController.removeListener(listener);
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
      children: [
        SingleChildScrollView(
          controller: scrollController,
          scrollDirection: Axis.horizontal,
          physics: const NeverScrollableScrollPhysics(),
          padding: const Pad(horizontal: 16),
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
                    height: labelHeight,
                    child: Row(
                      children: [
                        for (final span in monthSpans)
                          if (span.end - span.start > 1)
                            GestureDetector(
                              onTap: () {
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
                                width: (span.end - span.start + 1) * (cellSize + cellGap) - cellGap,
                                child: Center(
                                  child: Text(
                                    '${span.month}월',
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
                _buildActivityGrid(context, activities, lastToastedActivity, scrollController),
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
    ObjectRef<Activity?> lastToastedActivity,
    ScrollController scrollController,
  ) {
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

    Activity? getActivityAtPosition(Offset localPosition) {
      final weekIndex = (localPosition.dx / (cellSize + cellGap)).floor();
      final dayIndex = (localPosition.dy / (cellSize + cellGap)).floor();

      if (weekIndex >= 0 &&
          weekIndex < weeks.length &&
          dayIndex >= 0 &&
          dayIndex < 7 &&
          dayIndex < weeks[weekIndex].length) {
        final activity = weeks[weekIndex][dayIndex];
        if (activity.level != -1) {
          return activity;
        }
      }
      return null;
    }

    void showActivityToast(Activity? activity, {bool force = false}) {
      if (activity != null && (force || activity != lastToastedActivity.value)) {
        final date = activity.date.format(pattern: 'yyyy년 M월 d일');
        final message = activity.additions > 0 ? '$date에 ${activity.additions.comma}자를 작성했어요' : '$date에는 작성한 글이 없어요';
        context.toast(ToastType.success, message, bottom: 64);
        lastToastedActivity.value = activity;
      }
    }

    void handleDragStart(DragStartDetails details) {
      final activity = getActivityAtPosition(details.localPosition);
      showActivityToast(activity, force: true);
    }

    void handleDragUpdate(DragUpdateDetails details) {
      final activity = getActivityAtPosition(details.localPosition);
      showActivityToast(activity);
    }

    void handleDragEnd(_) {
      lastToastedActivity.value = null;
    }

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onVerticalDragStart: handleDragStart,
      onVerticalDragUpdate: handleDragUpdate,
      onVerticalDragEnd: handleDragEnd,
      onPanStart: handleDragStart,
      onPanUpdate: handleDragUpdate,
      onPanEnd: handleDragEnd,
      onTapDown: (details) {
        final activity = getActivityAtPosition(details.localPosition);
        showActivityToast(activity, force: true);
      },
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
                            : Container(
                                width: cellSize,
                                height: cellSize,
                                decoration: BoxDecoration(
                                  color: _getColorByLevel(context, weeks[week][day].level),
                                  borderRadius: BorderRadius.circular(2),
                                ),
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
      final key = change.date.format(pattern: 'yyyy-MM-dd');
      changesMap[key] = change;
    }

    final numbers = characterCountChanges.map((c) => c.additions).where((n) => n > 0).toList();

    final min = numbers.isEmpty ? 0 : numbers.reduce((a, b) => a < b ? a : b);
    final max = numbers.isEmpty ? 0 : numbers.reduce((a, b) => a > b ? a : b);
    final range = max - min;

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
        } else if (range == 0) {
          level = 3;
        } else {
          final value = (additions - min) / range;
          level = (value * 5).floor() + 1;
          if (level > 5) {
            level = 5;
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

    var currentDate = startDate.clone();
    while (currentDate.dayOfWeek != 7) {
      currentDate = currentDate.subtract(days: 1);
    }

    var weekIndex = 0;
    while (!currentDate.isAfter(endDate)) {
      final month = currentDate.month;

      if (monthSpans.isEmpty || monthSpans.last.month != month) {
        monthSpans.add(MonthSpan(month: month, start: weekIndex, end: weekIndex));
      } else {
        monthSpans.last.end = weekIndex;
      }

      currentDate = currentDate.add(days: 7);
      weekIndex++;
    }

    return monthSpans;
  }

  Color _getColorByLevel(BuildContext context, int level) {
    final isDark = context.theme.brightness == Brightness.dark;

    switch (level) {
      case 0:
        return isDark ? AppColors.dark.gray_800 : AppColors.gray_100;
      case 1:
        return isDark ? AppColors.dark.brand_700 : AppColors.brand_100;
      case 2:
        return isDark ? AppColors.dark.brand_600 : AppColors.brand_300;
      case 3:
        return isDark ? AppColors.dark.brand_500 : AppColors.brand_500;
      case 4:
        return isDark ? AppColors.dark.brand_300 : AppColors.brand_700;
      case 5:
        return isDark ? AppColors.dark.brand_100 : AppColors.brand_900;
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
