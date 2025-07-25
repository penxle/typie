import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/constants/plan_features.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/tappable.dart';

enum LimitBottomSheetType { limit, spellCheck }

class LimitBottomSheet extends StatelessWidget {
  const LimitBottomSheet({this.type = LimitBottomSheetType.limit, super.key});

  final LimitBottomSheetType type;

  @override
  Widget build(BuildContext context) {
    final List<IconData> icons = [
      LucideLightIcons.crown,
      LucideLightIcons.tag,
      LucideLightIcons.star,
      LucideLightIcons.key,
      LucideLightIcons.gift,
    ];

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Center(
            child: SizedBox(
              height: 32,
              width: 32 + (icons.length - 1) * 22,
              child: Stack(
                children: [
                  for (int i = 0; i < icons.length; i++)
                    Positioned(
                      left: i * 22,
                      child: Container(
                        decoration: BoxDecoration(
                          color: context.colors.surfaceDark,
                          border: Border.all(color: context.colors.surfaceDefault, width: 2),
                          borderRadius: BorderRadius.circular(999),
                        ),
                        padding: const Pad(all: 6),
                        child: Icon(icons[i], size: 16, color: context.colors.textBright),
                      ),
                    ),
                ],
              ),
            ),
          ),
          const Gap(16),
          const Text(
            '플랜 업그레이드가 필요해요',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          ),
          const Gap(4),
          if (type == LimitBottomSheetType.limit) ...[
            Text(
              '현재 플랜의 최대 사용량을 초과했어요.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 14, color: context.colors.textFaint),
            ),
            Text(
              '이어서 작성하려면 플랜을 업그레이드 해주세요.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 14, color: context.colors.textFaint),
            ),
          ] else
            Text(
              '맞춤법 검사는 유료 플랜에서 이용할 수 있어요.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 14, color: context.colors.textFaint),
            ),
          const Gap(16),
          Container(
            decoration: BoxDecoration(
              border: Border.all(color: context.colors.borderStrong),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Padding(
              padding: const Pad(all: 16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text('타이피 FULL ACCESS', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                  const Gap(12),
                  HorizontalDivider(color: context.colors.borderStrong),
                  const Gap(12),
                  Column(
                    spacing: 8,
                    children: fullPlanFeatures
                        .map((feature) => _FeatureItem(icon: feature.icon, label: feature.label))
                        .toList(),
                  ),
                ],
              ),
            ),
          ),
          const Gap(16),
          Tappable(
            onTap: () async {
              await context.router.root.maybePop();
              if (context.mounted) {
                await context.router.popAndPush(const EnrollPlanRoute());
              }
            },
            child: Container(
              decoration: BoxDecoration(color: context.colors.surfaceInverse, borderRadius: BorderRadius.circular(8)),
              padding: const Pad(vertical: 16),
              child: Text(
                '업그레이드',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: context.colors.textInverse),
                textAlign: TextAlign.center,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _FeatureItem extends StatelessWidget {
  const _FeatureItem({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 8,
      children: [
        Icon(icon, size: 16),
        Text(label, style: const TextStyle(fontSize: 14)),
      ],
    );
  }
}
