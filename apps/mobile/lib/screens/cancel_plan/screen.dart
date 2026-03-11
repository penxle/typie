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
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/cancel_plan/__generated__/cancel_plan_query.data.gql.dart';
import 'package:typie/screens/cancel_plan/__generated__/cancel_plan_query.req.gql.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

const _cardRadius = 12.0;
const _sectionGap = 16.0;

@RoutePage()
class CancelPlanScreen extends HookWidget {
  const CancelPlanScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final scrollController = useScrollController();

    return Screen(
      child: GraphQLOperation(
        initialBackgroundColor: context.colors.surfaceSubtle,
        operation: GCancelPlanScreen_QueryReq(),
        builder: (context, client, data) {
          final subscription = data.me?.subscription;
          if (subscription == null) {
            return const SizedBox.shrink();
          }

          return _Content(subscription: subscription, mixpanel: mixpanel, scrollController: scrollController);
        },
      ),
    );
  }
}

class _Content extends StatelessWidget {
  const _Content({required this.subscription, required this.mixpanel, required this.scrollController});

  final GCancelPlanScreen_QueryData_me_subscription subscription;
  final Mixpanel mixpanel;
  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    final bottomPadding = MediaQuery.paddingOf(context).bottom + 72;

    return Stack(
      children: [
        SingleChildScrollView(
          controller: scrollController,
          physics: const AlwaysScrollableScrollPhysics(),
          padding: EdgeInsets.fromLTRB(20, OverlayHeading.contentTopSpacing + 8, 20, bottomPadding),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text('이용권 해지', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
              const Gap(_sectionGap),
              DecoratedBox(
                decoration: _cardDecoration(context),
                child: const Padding(
                  padding: Pad(all: 18),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('정말 해지하시겠어요?', style: TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
                      Gap(6),
                      Text('해지 시 다음 혜택을 더 이상 받을 수 없어요.', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
                    ],
                  ),
                ),
              ),
              const Gap(_sectionGap),
              DecoratedBox(
                decoration: _cardDecoration(context),
                child: Padding(
                  padding: const Pad(all: 18),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('이용 중인 혜택', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
                      const Gap(12),
                      for (final feature in fullPlanFeatures) ...[
                        _FeatureItem(icon: feature.icon, label: feature.label),
                        if (feature != fullPlanFeatures.last) const Gap(10),
                      ],
                    ],
                  ),
                ),
              ),
              const Gap(12),
              Text(
                '지금 해지하더라도 ${subscription.expiresAt.toLocal().yyyyMMddKorean}까지는 계속해서 ${subscription.plan.name} 혜택을 이용할 수 있어요.',
                style: TextStyle(fontSize: 14, height: 1.5, color: context.colors.textFaint),
              ),
              const Gap(24),
              _PrimaryDangerButton(
                label: '스토어로 이동해서 해지하기',
                onTap: () async {
                  final url = Platform.isIOS
                      ? Uri.parse('https://apps.apple.com/account/subscriptions')
                      : Uri.parse('https://play.google.com/store/account/subscriptions?package=co.typie&sku=plan.full');

                  unawaited(mixpanel.track('cancel_plan_try'));
                  await launchUrl(url, mode: LaunchMode.externalApplication);
                },
              ),
              const Gap(8),
              _SecondaryButton(
                label: '계속 이용하기',
                onTap: () async {
                  await context.router.maybePop();
                },
              ),
            ],
          ),
        ),
        _Heading(scrollController: scrollController),
      ],
    );
  }
}

class _Heading extends StatelessWidget {
  const _Heading({required this.scrollController});

  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    return OverlayHeading(
      title: '이용권 해지',
      scrollController: scrollController,
      leading: Tappable(
        onTap: () async {
          await context.router.maybePop();
        },
        child: Tappable.scale(
          scale: 0.95,
          child: SizedBox(
            width: 36,
            height: 36,
            child: Align(
              alignment: Alignment.centerLeft,
              child: Icon(LucideLightIcons.chevron_left, size: 22, color: context.colors.textDefault),
            ),
          ),
        ),
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
      spacing: 10,
      children: [
        Icon(icon, size: 18, color: context.colors.textSubtle),
        Expanded(
          child: Text(label, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
        ),
      ],
    );
  }
}

class _PrimaryDangerButton extends StatelessWidget {
  const _PrimaryDangerButton({required this.label, required this.onTap});

  final String label;
  final Future<void> Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () async {
        await onTap();
      },
      child: DecoratedBox(
        decoration: BoxDecoration(color: context.colors.accentDanger, borderRadius: BorderRadius.circular(10)),
        child: Tappable.scale(
          child: Padding(
            padding: const Pad(vertical: 14),
            child: Center(
              child: Text(
                label,
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: context.colors.textBright),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _SecondaryButton extends StatelessWidget {
  const _SecondaryButton({required this.label, required this.onTap});

  final String label;
  final Future<void> Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () async {
        await onTap();
      },
      child: DecoratedBox(
        decoration: BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(10)),
        child: Tappable.scale(
          child: Padding(
            padding: const Pad(vertical: 14),
            child: Center(
              child: Text(
                label,
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

BoxDecoration _cardDecoration(BuildContext context) =>
    BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(_cardRadius));
