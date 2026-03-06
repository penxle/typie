import 'package:jiffy/jiffy.dart';

const weekdayLabels = ['일', '월', '화', '수', '목', '금', '토'];

class StatsCharacterCountChange {
  const StatsCharacterCountChange({required this.date, required this.additions, required this.deletions});

  final Jiffy date;
  final int additions;
  final int deletions;
}

class StreakData {
  const StreakData({
    required this.currentStreak,
    required this.longestStreak,
    required this.thisMonthDays,
    required this.totalDays,
    required this.avgCharactersPerDay,
  });

  final int currentStreak;
  final int longestStreak;
  final int thisMonthDays;
  final int totalDays;
  final int avgCharactersPerDay;
}

class WeekdayData {
  const WeekdayData({
    required this.dayIndex,
    required this.label,
    required this.totalAdditions,
    required this.avgAdditions,
    required this.count,
  });

  final int dayIndex;
  final String label;
  final int totalAdditions;
  final int avgAdditions;
  final int count;
}

StreakData calculateStreakData(
  List<StatsCharacterCountChange> characterCountChanges,
  int totalCharacterCount, {
  Jiffy? now,
}) {
  final today = (now ?? Jiffy.now()).clone();
  final activeDates = {
    for (final change in characterCountChanges)
      if (change.additions > 0) _dateKey(change.date.toLocal()),
  };

  var currentStreak = 0;
  var checkDate = today;

  if (!activeDates.contains(_dateKey(today))) {
    checkDate = checkDate.subtract(days: 1);
  }

  while (activeDates.contains(_dateKey(checkDate))) {
    currentStreak++;
    checkDate = checkDate.subtract(days: 1);
  }

  var longestStreak = 0;
  var tempStreak = 0;
  final sortedDates = activeDates.toList()..sort();

  for (var i = 0; i < sortedDates.length; i++) {
    if (i == 0) {
      tempStreak = 1;
    } else {
      final prevDate = Jiffy.parse(sortedDates[i - 1]);
      final currDate = Jiffy.parse(sortedDates[i]);
      if (currDate.diff(prevDate, unit: Unit.day) == 1) {
        tempStreak++;
      } else {
        tempStreak = 1;
      }
    }
    if (tempStreak > longestStreak) {
      longestStreak = tempStreak;
    }
  }

  var thisMonthDays = 0;
  for (final date in activeDates) {
    final parsed = Jiffy.parse(date);
    if (parsed.year == today.year && parsed.month == today.month) {
      thisMonthDays++;
    }
  }

  final totalDays = activeDates.length;
  final avgCharactersPerDay = totalDays > 0 ? (totalCharacterCount / totalDays).round() : 0;

  return StreakData(
    currentStreak: currentStreak,
    longestStreak: longestStreak,
    thisMonthDays: thisMonthDays,
    totalDays: totalDays,
    avgCharactersPerDay: avgCharactersPerDay,
  );
}

List<WeekdayData> calculateWeekdayPattern(List<StatsCharacterCountChange> characterCountChanges) {
  final weekdayStats = List.generate(7, (index) => _WeekdayAccumulator(dayIndex: index, label: weekdayLabels[index]));

  for (final change in characterCountChanges) {
    if (change.additions <= 0) {
      continue;
    }

    final dayOfWeek = _toDayIndex(change.date.toLocal());
    weekdayStats[dayOfWeek]
      ..totalAdditions += change.additions
      ..count += 1;
  }

  return [
    for (final stat in weekdayStats)
      WeekdayData(
        dayIndex: stat.dayIndex,
        label: stat.label,
        totalAdditions: stat.totalAdditions,
        avgAdditions: stat.count > 0 ? (stat.totalAdditions / stat.count).round() : 0,
        count: stat.count,
      ),
  ];
}

String _dateKey(Jiffy date) => date.format(pattern: 'yyyy-MM-dd');

int _toDayIndex(Jiffy date) {
  final weekday = DateTime(date.year, date.month, date.date).weekday;
  return weekday % 7;
}

class _WeekdayAccumulator {
  _WeekdayAccumulator({required this.dayIndex, required this.label});

  final int dayIndex;
  final String label;
  int totalAdditions = 0;
  int count = 0;
}
