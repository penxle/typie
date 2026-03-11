import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/current_plan/__generated__/current_plan_query.data.gql.dart';
import 'package:typie/screens/current_plan/__generated__/current_plan_query.req.gql.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';

const _cardRadius = 12.0;
const _sectionGap = 16.0;

@RoutePage()
class CurrentPlanScreen extends HookWidget {
  const CurrentPlanScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final scrollController = useScrollController();

    return Screen(
      child: GraphQLOperation(
        initialBackgroundColor: context.colors.surfaceSubtle,
        operation: GCurrentPlanScreen_QueryReq(),
        builder: (context, client, data) {
          final subscription = data.me?.subscription;
          if (subscription == null) {
            return const SizedBox.shrink();
          }

          return _Content(subscription: subscription, scrollController: scrollController);
        },
      ),
    );
  }
}

class _Content extends StatelessWidget {
  const _Content({required this.subscription, required this.scrollController});

  final GCurrentPlanScreen_QueryData_me_subscription subscription;
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
              const Text('이용권 정보', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
              const Gap(_sectionGap),
              DecoratedBox(
                decoration: _cardDecoration(context),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Padding(
                      padding: const Pad(all: 18),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('현재 이용권', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
                          const Gap(6),
                          Text(
                            subscription.plan.name,
                            style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700),
                          ),
                          const Gap(12),
                          ..._detailLines(context, subscription),
                        ],
                      ),
                    ),
                    ..._footer(context, subscription),
                  ],
                ),
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
      title: '이용권 정보',
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

List<Widget> _detailLines(BuildContext context, GCurrentPlanScreen_QueryData_me_subscription subscription) {
  if (subscription.plan.availability == GPlanAvailability.TRIAL) {
    return [
      Text(
        '무료 체험이 ${subscription.expiresAt.toLocal().yyyyMMdd}에 종료돼요.',
        style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
      ),
    ];
  }

  final lines = <String>[
    '이용권 가격: ${subscription.plan.fee.comma}원',
    if (subscription.state == GSubscriptionState.ACTIVE)
      '다음 결제일: ${subscription.expiresAt.toLocal().yyyyMMdd}'
    else
      '해지 예정일: ${subscription.expiresAt.toLocal().yyyyMMdd}',
  ];

  return [
    for (final line in lines)
      Padding(
        padding: const Pad(bottom: 3),
        child: Text(
          line,
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
        ),
      ),
  ];
}

List<Widget> _footer(BuildContext context, GCurrentPlanScreen_QueryData_me_subscription subscription) {
  if (subscription.plan.availability == GPlanAvailability.IN_APP_PURCHASE) {
    return [
      HorizontalDivider(color: context.colors.borderSubtle),
      SizedBox(
        height: 54,
        child: Row(
          children: [
            Expanded(
              child: _FooterAction(
                label: '해지하기',
                onTap: () async {
                  await context.router.push(const CancelPlanRoute());
                },
              ),
            ),
            AppVerticalDivider(color: context.colors.borderSubtle),
            Expanded(
              child: _FooterAction(
                label: '변경하기',
                onTap: () async {
                  await context.router.push(const EnrollPlanRoute());
                },
              ),
            ),
          ],
        ),
      ),
    ];
  }

  if (subscription.plan.availability == GPlanAvailability.BILLING_KEY) {
    return [
      HorizontalDivider(color: context.colors.borderSubtle),
      const _FooterNote('웹사이트에서 가입한 이용권이에요.\n정보 변경이 필요할 경우 웹사이트에서 진행해주세요.'),
    ];
  }

  if (subscription.plan.availability == GPlanAvailability.MANUAL) {
    return [
      HorizontalDivider(color: context.colors.borderSubtle),
      const _FooterNote('정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요.'),
    ];
  }

  if (subscription.plan.availability == GPlanAvailability.TRIAL) {
    return [
      HorizontalDivider(color: context.colors.borderSubtle),
      Padding(
        padding: const Pad(all: 16),
        child: Tappable(
          onTap: () async {
            await context.router.push(const EnrollPlanRoute());
          },
          child: DecoratedBox(
            decoration: BoxDecoration(color: context.colors.surfaceInverse, borderRadius: BorderRadius.circular(10)),
            child: Tappable.scale(
              child: Padding(
                padding: const Pad(vertical: 13),
                child: Center(
                  child: Text(
                    '지금 업그레이드',
                    style: TextStyle(fontSize: 15, fontWeight: FontWeight.w700, color: context.colors.textInverse),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    ];
  }

  return const [];
}

class _FooterAction extends StatelessWidget {
  const _FooterAction({required this.label, required this.onTap});

  final String label;
  final Future<void> Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () async {
        await onTap();
      },
      child: Tappable.scale(
        child: Center(
          child: Text(
            label,
            style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
          ),
        ),
      ),
    );
  }
}

class _FooterNote extends StatelessWidget {
  const _FooterNote(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const Pad(all: 16),
      child: Text(text, style: TextStyle(fontSize: 14, height: 1.5, color: context.colors.textFaint)),
    );
  }
}

BoxDecoration _cardDecoration(BuildContext context) =>
    BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(_cardRadius));
