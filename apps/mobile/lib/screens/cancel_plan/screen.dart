import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/cancel_plan/__generated__/screen.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

@RoutePage()
class CancelPlanScreen extends StatelessWidget {
  const CancelPlanScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: const Heading(title: '이용권 해지'),
      padding: const Pad(horizontal: 20, top: 40),
      child: GraphQLOperation(
        operation: GCancelPlanScreen_QueryReq(),
        builder: (context, client, data) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                '정말 해지하시겠어요?',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
              ),
              const Gap(4),
              const Text(
                '해지 시 다음 혜택을 더 이상 받을 수 없어요',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 14, color: AppColors.gray_500),
              ),
              const Gap(24),
              Container(
                decoration: BoxDecoration(
                  border: Border.all(color: AppColors.gray_950),
                  borderRadius: BorderRadius.circular(8),
                  color: AppColors.white,
                ),
                padding: const Pad(all: 16),
                child: const Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 8,
                  children: [
                    Text('이용중인 혜택', style: TextStyle(fontSize: 14, color: AppColors.gray_500)),
                    _FeatureItem(icon: LucideLightIcons.book_open_text, label: '무제한 글자 수'),
                    _FeatureItem(icon: LucideLightIcons.images, label: '무제한 파일 업로드'),
                    _FeatureItem(icon: LucideLightIcons.link, label: '커스텀 공유 주소'),
                    _FeatureItem(icon: LucideLightIcons.flask_conical, label: '베타 기능 우선 접근'),
                    _FeatureItem(icon: LucideLightIcons.headset, label: '문제 발생 시 우선 지원'),
                    _FeatureItem(icon: LucideLightIcons.sprout, label: '디스코드 커뮤니티 참여'),
                    _FeatureItem(icon: LucideLightIcons.ellipsis, label: '그리고 더 많은 혜택'),
                  ],
                ),
              ),
              const Gap(8),
              Text(
                '지금 해지하더라도 ${data.me!.plan!.expiresAt.toLocal().subtract(days: 1).format(pattern: 'yyyy년 MM월 dd일')}까지는 계속해서 타이피 FULL ACCESS 혜택을 이용할 수 있어요.',
                style: const TextStyle(fontSize: 14, color: AppColors.gray_500),
              ),
              const Gap(24),
              Tappable(
                onTap: () async {
                  final url = Platform.isIOS
                      ? Uri.parse('https://apps.apple.com/account/subscriptions')
                      : Uri.parse('https://play.google.com/store/account/subscriptions?package=co.typie&sku=plan.full');

                  await launchUrl(url, mode: LaunchMode.externalApplication);
                },
                child: Container(
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    border: Border.all(color: AppColors.gray_950),
                    borderRadius: BorderRadius.circular(8),
                    color: AppColors.red_500,
                  ),
                  padding: const Pad(vertical: 12),
                  child: const Text(
                    '스토어로 이동해서 해지하기',
                    style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.white),
                  ),
                ),
              ),
              const Gap(8),
              Tappable(
                onTap: () async {
                  await context.router.maybePop();
                },
                child: Container(
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    border: Border.all(color: AppColors.gray_950),
                    borderRadius: BorderRadius.circular(8),
                    color: AppColors.white,
                  ),
                  padding: const Pad(vertical: 12),
                  child: const Text('계속 이용하기', style: TextStyle(fontSize: 16)),
                ),
              ),
            ],
          );
        },
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
