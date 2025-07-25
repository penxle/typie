import 'dart:async';
import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/constants/plan_features.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/cancel_plan/__generated__/cancel_plan_query.req.gql.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

@RoutePage()
class CancelPlanScreen extends HookWidget {
  const CancelPlanScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
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
              Text(
                '해지 시 다음 혜택을 더 이상 받을 수 없어요',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 14, color: context.colors.textFaint),
              ),
              const Gap(24),
              Container(
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong),
                  borderRadius: BorderRadius.circular(8),
                  color: context.colors.surfaceDefault,
                ),
                padding: const Pad(all: 16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 8,
                  children: [
                    Text('이용중인 혜택', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
                    ...fullPlanFeatures.map((feature) => _FeatureItem(icon: feature.icon, label: feature.label)),
                  ],
                ),
              ),
              const Gap(8),
              Text(
                '지금 해지하더라도 ${data.me!.subscription!.expiresAt.toLocal().subtract(days: 1).yyyyMMddKorean}까지는 계속해서 ${data.me!.subscription!.plan.name} 혜택을 이용할 수 있어요.',
                style: TextStyle(fontSize: 14, color: context.colors.textFaint),
              ),
              const Gap(24),
              Tappable(
                onTap: () async {
                  final url = Platform.isIOS
                      ? Uri.parse('https://apps.apple.com/account/subscriptions')
                      : Uri.parse('https://play.google.com/store/account/subscriptions?package=co.typie&sku=plan.full');

                  unawaited(mixpanel.track('cancel_plan_try'));
                  await launchUrl(url, mode: LaunchMode.externalApplication);
                },
                child: Container(
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    border: Border.all(color: context.colors.borderStrong),
                    borderRadius: BorderRadius.circular(8),
                    color: context.colors.accentDanger,
                  ),
                  padding: const Pad(vertical: 12),
                  child: Text(
                    '스토어로 이동해서 해지하기',
                    style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textBright),
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
                    border: Border.all(color: context.colors.borderStrong),
                    borderRadius: BorderRadius.circular(8),
                    color: context.colors.surfaceDefault,
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
