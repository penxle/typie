import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/screens/profile/__generated__/profile_query.data.gql.dart';
import 'package:typie/styles/colors.dart';

class ActivityGrid extends HookWidget {
  const ActivityGrid({super.key, required this.characterCountChanges});

  final List<GProfileScreen_QueryData_me_characterCountChanges> characterCountChanges;

  static const cellSize = 13.0;
  static const cellGap = 3.0;
  static const labelHeight = 16.0;

  @override
  Widget build(BuildContext context) {
    final scrollController = useScrollController();

    useAsyncEffect(() async {
      scrollController.jumpTo(scrollController.position.maxScrollExtent);

      return null;
    }, []);

    final endDate = Jiffy.now();
    final startDate = endDate.subtract(days: 364);

    final activities = useMemoized(() => _generateActivities(startDate, endDate), [characterCountChanges]);
    final monthSpans = useMemoized(() => _generateMonthSpans(startDate, endDate), [characterCountChanges]);

    final weekCount = ((endDate.diff(startDate, unit: Unit.day) + 1) / 7).ceil();
    final totalWidth = weekCount * (cellSize + cellGap) - cellGap;

    return SingleChildScrollView(
      controller: scrollController,
      scrollDirection: Axis.horizontal,
      child: SizedBox(
        width: totalWidth,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              height: labelHeight,
              child: Row(
                children: [
                  for (final span in monthSpans)
                    if (span.end - span.start > 1)
                      SizedBox(
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
                ],
              ),
            ),
            const SizedBox(height: cellGap),
            _buildActivityGrid(context, activities),
          ],
        ),
      ),
    );
  }

  Widget _buildActivityGrid(BuildContext context, List<Activity> activities) {
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

    return Row(
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
