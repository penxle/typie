import 'package:flutter_test/flutter_test.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/screens/stats/stats_calculator.dart';

void main() {
  test('calculateStreakData computes streaks and monthly counts', () {
    final changes = [
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-06'), additions: 120, deletions: 5),
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-05'), additions: 80, deletions: 3),
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-03'), additions: 40, deletions: 1),
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-01'), additions: 20, deletions: 0),
      StatsCharacterCountChange(date: Jiffy.parse('2026-02-28'), additions: 10, deletions: 0),
    ];

    final data = calculateStreakData(changes, 1000, now: Jiffy.parse('2026-03-06'));

    expect(data.currentStreak, 2);
    expect(data.longestStreak, 2);
    expect(data.thisMonthDays, 4);
    expect(data.totalDays, 5);
    expect(data.avgCharactersPerDay, 200);
  });

  test('calculateStreakData starts from yesterday when today is inactive', () {
    final changes = [
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-05'), additions: 90, deletions: 0),
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-04'), additions: 70, deletions: 0),
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-02'), additions: 10, deletions: 0),
    ];

    final data = calculateStreakData(changes, 300, now: Jiffy.parse('2026-03-06'));

    expect(data.currentStreak, 2);
    expect(data.longestStreak, 2);
    expect(data.thisMonthDays, 3);
    expect(data.totalDays, 3);
    expect(data.avgCharactersPerDay, 100);
  });

  test('calculateWeekdayPattern returns sunday-based indices and averages', () {
    final changes = [
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-01'), additions: 10, deletions: 0), // 일
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-08'), additions: 30, deletions: 0), // 일
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-02'), additions: 20, deletions: 0), // 월
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-09'), additions: 40, deletions: 0), // 월
      StatsCharacterCountChange(date: Jiffy.parse('2026-03-11'), additions: 0, deletions: 0),
    ];

    final data = calculateWeekdayPattern(changes);

    expect(data, hasLength(7));
    expect(data[0].label, '일');
    expect(data[0].count, 2);
    expect(data[0].avgAdditions, 20);
    expect(data[1].label, '월');
    expect(data[1].count, 2);
    expect(data[1].avgAdditions, 30);
    expect(data[3].count, 0);
    expect(data[3].avgAdditions, 0);
  });
}
