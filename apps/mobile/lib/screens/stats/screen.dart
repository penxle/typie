import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gal/gal.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/stats/__generated__/generate_activity_image_mutation.req.gql.dart';
import 'package:typie/screens/stats/__generated__/stats_query.req.gql.dart';
import 'package:typie/screens/stats/activity_chart.dart';
import 'package:typie/screens/stats/stats_calculator.dart';
import 'package:typie/widgets/activity_grid.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/popover/list.dart';
import 'package:typie/widgets/popover/popover.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class StatsScreen extends StatelessWidget {
  const StatsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: const Heading(title: '나의 글쓰기 통계'),
      child: GraphQLOperation(
        operation: GStatsScreen_QueryReq(),
        builder: (context, client, data) {
          final user = data.me;
          if (user == null) {
            return const SizedBox.shrink();
          }

          final changes = [
            for (final change in user.characterCountChanges)
              StatsCharacterCountChange(date: change.date, additions: change.additions, deletions: change.deletions),
          ];

          final totalCharacterCount = user.usage.totalCharacterCount;
          final streakData = calculateStreakData(changes, totalCharacterCount);
          final weekdayData = calculateWeekdayPattern(changes);
          final maxWeekdayAvg = weekdayData.isEmpty ? 0 : weekdayData.map((day) => day.avgAdditions).reduce(max);
          final bestWeekdayIndex = maxWeekdayAvg > 0
              ? weekdayData.indexWhere((day) => day.avgAdditions == maxWeekdayAvg)
              : -1;
          final bottomPadding = MediaQuery.paddingOf(context).bottom + 72;

          Future<void> copyActivityImage() async {
            try {
              final response = await client.request(GStatsScreen_GenerateActivityImage_MutationReq());
              final bytes = base64Decode(response.generateActivityImage.value);
              final clipboard = SystemClipboard.instance;

              if (clipboard == null) {
                throw StateError('Clipboard is not available');
              }

              final item = DataWriterItem(suggestedName: '${user.name}-나의-글쓰기-발자취.png')..add(Formats.png(bytes));
              await clipboard.write([item]);

              if (context.mounted) {
                context.toast(ToastType.success, '이미지가 클립보드에 복사되었어요.');
              }
            } catch (_) {
              if (context.mounted) {
                context.toast(ToastType.error, '이미지를 복사할 수 없어요.');
              }
            }
          }

          Future<void> downloadActivityImage() async {
            try {
              final response = await client.request(GStatsScreen_GenerateActivityImage_MutationReq());
              final bytes = base64Decode(response.generateActivityImage.value);
              final name = '${user.name}-나의-글쓰기-발자취';

              await Gal.putImageBytes(bytes, name: name);

              if (context.mounted) {
                context.toast(ToastType.success, '이미지가 기기에 저장되었어요.');
              }
            } on GalException catch (e) {
              if (!context.mounted) {
                return;
              }

              if (e.type == GalExceptionType.accessDenied) {
                context.toast(ToastType.error, '사진 접근 권한이 필요해요.');
              } else {
                context.toast(ToastType.error, '이미지를 저장할 수 없어요.');
              }
            } catch (_) {
              if (context.mounted) {
                context.toast(ToastType.error, '이미지를 저장할 수 없어요.');
              }
            }
          }

          return SingleChildScrollView(
            padding: EdgeInsets.fromLTRB(20, 20, 20, bottomPadding),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 16,
              children: [
                _SummaryCard(label: '총 글자', value: totalCharacterCount.comma, unit: '자'),
                IntrinsicHeight(
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Expanded(
                        child: _SummaryCard(label: '총 문서', value: user.documentCount.toString(), unit: '개'),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: _SummaryCard(label: '활동일', value: streakData.totalDays.toString(), unit: '일'),
                      ),
                    ],
                  ),
                ),
                _StreakCard(streakData: streakData),
                _WeekdayCard(
                  weekdayData: weekdayData,
                  maxWeekdayAvg: maxWeekdayAvg,
                  bestWeekdayIndex: bestWeekdayIndex,
                ),
                _SectionCard(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Row(
                        children: [
                          Text(
                            '지난 1년간의 기록',
                            style: TextStyle(
                              fontSize: 14,
                              fontWeight: FontWeight.w600,
                              color: context.colors.textSubtle,
                            ),
                          ),
                          const Spacer(),
                          Popover(
                            screenPadding: const EdgeInsets.all(20),
                            collapsedBorderRadius: BorderRadius.circular(8),
                            backgroundColor: context.colors.surfaceDefault,
                            borderSide: BorderSide(color: context.colors.borderStrong),
                            anchor: const _ActionButton(child: _ActionButtonContent(label: '이미지 받기')),
                            pane: _ActivityImagePane(
                              onCopy: () {
                                unawaited(copyActivityImage());
                              },
                              onDownload: () {
                                unawaited(downloadActivityImage());
                              },
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      ActivityGrid(
                        changes: [
                          for (final change in changes)
                            ActivityGridChange(date: change.date, additions: change.additions),
                        ],
                      ),
                    ],
                  ),
                ),
                _SectionCard(child: ActivityChart(characterCountChanges: changes)),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _SummaryCard extends StatelessWidget {
  const _SummaryCard({required this.label, required this.value, required this.unit});

  final String label;
  final String value;
  final String unit;

  @override
  Widget build(BuildContext context) {
    return _SectionCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
          ),
          const SizedBox(height: 8),
          SizedBox(
            height: 44,
            child: Align(
              alignment: Alignment.bottomLeft,
              child: FittedBox(
                fit: BoxFit.scaleDown,
                alignment: Alignment.centerLeft,
                child: RichText(
                  text: TextSpan(
                    text: value,
                    style: TextStyle(fontSize: 28, fontWeight: FontWeight.w700, color: context.colors.textDefault),
                    children: [
                      TextSpan(
                        text: unit,
                        style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: context.colors.textFaint),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _StreakCard extends StatelessWidget {
  const _StreakCard({required this.streakData});

  final StreakData streakData;

  @override
  Widget build(BuildContext context) {
    return _SectionCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '연속 기록',
            style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
          ),
          const SizedBox(height: 8),
          RichText(
            text: TextSpan(
              text: streakData.currentStreak.toString(),
              style: TextStyle(fontSize: 32, fontWeight: FontWeight.w700, color: context.colors.textDefault),
              children: [
                TextSpan(
                  text: '일째',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: context.colors.textFaint),
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Container(height: 1, color: context.colors.borderDefault),
          const SizedBox(height: 12),
          Row(
            children: [
              Text('최장 ', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${streakData.longestStreak}일',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
              ),
              const SizedBox(width: 12),
              Text('이번 달 ', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${streakData.thisMonthDays}일',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _WeekdayCard extends StatelessWidget {
  const _WeekdayCard({required this.weekdayData, required this.maxWeekdayAvg, required this.bestWeekdayIndex});

  final List<WeekdayData> weekdayData;
  final int maxWeekdayAvg;
  final int bestWeekdayIndex;

  @override
  Widget build(BuildContext context) {
    final safeMaxWeekdayAvg = max(maxWeekdayAvg, 1);

    return _SectionCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                '요일별 기록',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
              ),
              const Spacer(),
              if (bestWeekdayIndex >= 0)
                Text(
                  '${weekdayLabels[bestWeekdayIndex]}요일 최다',
                  style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                ),
            ],
          ),
          const SizedBox(height: 16),
          SizedBox(
            height: 52,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                for (var index = 0; index < weekdayData.length; index++) ...[
                  Flexible(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        Expanded(
                          child: Align(
                            alignment: Alignment.bottomCenter,
                            child: Container(
                              width: double.infinity,
                              height: max((weekdayData[index].avgAdditions / safeMaxWeekdayAvg) * 32, 2),
                              decoration: BoxDecoration(
                                color: weekdayData[index].dayIndex == bestWeekdayIndex
                                    ? context.colors.textDefault
                                    : context.colors.borderDefault,
                                borderRadius: BorderRadius.circular(3),
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(height: 6),
                        Text(
                          weekdayData[index].label,
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w500,
                            color: weekdayData[index].dayIndex == bestWeekdayIndex
                                ? context.colors.textDefault
                                : context.colors.textFaint,
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (index < weekdayData.length - 1) const SizedBox(width: 8),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: ShapeDecoration(
        color: context.colors.surfaceDefault,
        shape: RoundedSuperellipseBorder(
          borderRadius: BorderRadius.circular(8),
          side: BorderSide(color: context.colors.borderStrong),
        ),
      ),
      child: Padding(padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7), child: child),
    );
  }
}

class _ActionButtonContent extends StatelessWidget {
  const _ActionButtonContent({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Text(
      label,
      style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
    );
  }
}

class _ActivityImagePane extends StatelessWidget {
  const _ActivityImagePane({required this.onCopy, required this.onDownload});

  final VoidCallback onCopy;
  final VoidCallback onDownload;

  @override
  Widget build(BuildContext context) {
    return IntrinsicWidth(
      child: Padding(
        padding: const EdgeInsets.all(Popover.panePadding),
        child: PopoverList(
          items: [
            PopoverListItem(
              onSelected: () {
                Popover.close(context);
                onCopy();
              },
              child: const _ActivityImagePaneItem(icon: LucideLightIcons.copy, label: '클립보드에 복사'),
            ),
            PopoverListItem(
              onSelected: () {
                Popover.close(context);
                onDownload();
              },
              child: const _ActivityImagePaneItem(icon: LucideLightIcons.download, label: '기기에 저장'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ActivityImagePaneItem extends StatelessWidget {
  const _ActivityImagePaneItem({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Row(
        spacing: 12,
        children: [
          Icon(icon, size: 18, color: context.colors.textDefault),
          Expanded(
            child: Text(
              label,
              style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textDefault),
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: context.colors.surfaceDefault,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: context.colors.borderStrong),
      ),
      padding: const Pad(all: 16),
      child: child,
    );
  }
}
