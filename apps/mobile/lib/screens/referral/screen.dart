import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:share_plus/share_plus.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/referral/__generated__/referral_mutation.req.gql.dart';
import 'package:typie/screens/referral/__generated__/referral_query.data.gql.dart';
import 'package:typie/screens/referral/__generated__/referral_query.req.gql.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

const _cardRadius = 12.0;
const _sectionGap = 16.0;

@RoutePage()
class ReferralScreen extends HookWidget {
  const ReferralScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GReferralScreen_QueryReq(),
      builder: (context, client, data) {
        return _Content(data: data, client: client);
      },
    );
  }
}

class _Content extends HookWidget {
  const _Content({required this.data, required this.client});

  final GReferralScreen_QueryData data;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final scrollController = useScrollController();
    final referralCount = data.me?.referrals.length ?? 0;
    final compensatedCount = data.me?.referrals.where((r) => r.compensated).length ?? 0;
    final bottomPadding = MediaQuery.paddingOf(context).bottom + 72;

    Future<void> copyLink() async {
      final issueReferralUrl = GReferralScreen_IssueReferralUrl_MutationReq();
      final response = await client.request(issueReferralUrl);
      final referralUrl = response.issueReferralUrl;
      final message = '📝 타이피 가입하고 한달 무료 혜택 받아가세요! $referralUrl';

      await Clipboard.setData(ClipboardData(text: message));

      if (context.mounted) {
        context.toast(ToastType.success, '초대 링크가 복사되었어요.');
      }
    }

    Future<void> shareLink() async {
      final issueReferralUrl = GReferralScreen_IssueReferralUrl_MutationReq();
      final response = await client.request(issueReferralUrl);

      if (!context.mounted) {
        return;
      }

      final referralUrl = response.issueReferralUrl;
      final message = '📝 타이피 가입하고 한달 무료 혜택 받아가세요! $referralUrl';
      final box = context.findRenderObject() as RenderBox?;

      await SharePlus.instance.share(
        ShareParams(title: message, text: message, sharePositionOrigin: box!.localToGlobal(Offset.zero) & box.size),
      );
    }

    return Screen(
      extendBodyBehindAppBar: true,
      heading: _Heading(scrollController: scrollController),
      child: OverlayHeadingLayout(
        child: SingleChildScrollView(
          controller: scrollController,
          physics: const AlwaysScrollableScrollPhysics(),
          padding: EdgeInsets.fromLTRB(20, 0, 20, bottomPadding),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: EdgeInsets.only(top: OverlayHeading.titleTopPadding(context), bottom: 4),
                child: const Text('초대', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
              ),
              const Gap(_sectionGap),
              DecoratedBox(
                decoration: _cardDecoration(context),
                child: Padding(
                  padding: const Pad(all: 18),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text('친구 초대', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
                      const Gap(8),
                      Text(
                        '친구를 초대하면 친구는 바로 1달 무료, 친구가 첫 결제를 하면 나도 1달 무료 혜택을 받아요.',
                        style: TextStyle(fontSize: 14, height: 1.5, color: context.colors.textSubtle),
                      ),
                      const Gap(16),
                      Row(
                        spacing: 8,
                        children: [
                          Expanded(
                            child: _ActionButton(icon: LucideLightIcons.copy, label: '초대 링크 복사', onTap: copyLink),
                          ),
                          Expanded(
                            child: _ActionButton(icon: LucideLightIcons.share, label: '공유하기', onTap: shareLink),
                          ),
                        ],
                      ),
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
                      const Text('초대 현황', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
                      const Gap(14),
                      _MetricCard(icon: LucideLightIcons.users, label: '초대한 친구', value: '$referralCount명'),
                      const Gap(10),
                      _MetricCard(
                        icon: LucideLightIcons.gift,
                        label: '받은 혜택',
                        value: compensatedCount > 0 ? '$compensatedCount개월 무료' : '없음',
                      ),
                    ],
                  ),
                ),
              ),
              const _SectionLabel(text: '초대 혜택 안내', top: 24),
              DecoratedBox(
                decoration: _cardDecoration(context),
                child: const Padding(
                  padding: Pad(all: 18),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      _BulletPoint(text: '초대 링크를 통해 웹에서 가입하고, 웹에서 플랜을 가입해야 초대 혜택을 받을 수 있어요. 앱에서 가입하면 혜택을 받을 수 없어요.'),
                      Gap(10),
                      _BulletPoint(
                        text:
                            '친구가 초대 링크로 가입하면 친구는 즉시 FULL ACCESS 플랜 1개월에 해당하는 크레딧을 지급받아요. 지급받은 크레딧으로 바로 FULL ACCESS 플랜을 체험해볼 수 있어요.',
                      ),
                      Gap(10),
                      _BulletPoint(
                        text:
                            '친구가 크레딧을 통한 체험을 끝내고 첫 결제를 완료하면 나도 FULL ACCESS 플랜 1개월에 상응하는 크레딧을 지급받아요. 이 크레딧은 다음 FULL ACCESS 플랜 갱신시 자동으로 이용돼요.',
                      ),
                      Gap(10),
                      _BulletPoint(text: '초대 횟수에는 제한이 없어요.'),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Heading extends StatelessWidget implements PreferredSizeWidget {
  const _Heading({required this.scrollController});

  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    return OverlayHeading(
      title: '초대',
      scrollController: scrollController,
      leading: OverlayHeadingBackButton(
        onTap: () async {
          await context.router.maybePop();
        },
      ),
    );
  }

  @override
  Size get preferredSize => const Size.fromHeight(OverlayHeading.height);
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({required this.icon, required this.label, required this.onTap});

  final IconData icon;
  final String label;
  final Future<void> Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () async {
        await onTap();
      },
      child: DecoratedBox(
        decoration: BoxDecoration(color: context.colors.surfaceSubtle, borderRadius: BorderRadius.circular(10)),
        child: Tappable.scale(
          child: Padding(
            padding: const Pad(horizontal: 12, vertical: 12),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              spacing: 8,
              children: [
                Icon(icon, size: 16, color: context.colors.textSubtle),
                Flexible(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _MetricCard extends StatelessWidget {
  const _MetricCard({required this.icon, required this.label, required this.value});

  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(color: context.colors.surfaceSubtle, borderRadius: BorderRadius.circular(10)),
      child: Padding(
        padding: const Pad(all: 16),
        child: Row(
          children: [
            Icon(icon, size: 18, color: context.colors.textSubtle),
            const Gap(8),
            Expanded(
              child: Text(label, style: TextStyle(fontSize: 15, color: context.colors.textSubtle)),
            ),
            Text(value, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          ],
        ),
      ),
    );
  }
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel({required this.text, this.top = 20});

  final String text;
  final double top;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: Pad(top: top, bottom: 12),
      child: Text(
        text,
        style: TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w700,
          letterSpacing: 0.3,
          color: context.colors.textFaint,
        ),
      ),
    );
  }
}

class _BulletPoint extends StatelessWidget {
  const _BulletPoint({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          margin: const EdgeInsets.only(top: 9),
          width: 4,
          height: 4,
          decoration: BoxDecoration(color: context.colors.textFaint, shape: BoxShape.circle),
        ),
        const Gap(8),
        Expanded(
          child: Text(text, style: TextStyle(fontSize: 14, color: context.colors.textFaint, height: 1.6)),
        ),
      ],
    );
  }
}

BoxDecoration _cardDecoration(BuildContext context) =>
    BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(_cardRadius));
