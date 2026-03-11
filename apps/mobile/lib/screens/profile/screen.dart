import 'dart:math' as math;

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/profile/__generated__/profile_query.data.gql.dart';
import 'package:typie/screens/profile/__generated__/profile_query.req.gql.dart';
import 'package:typie/screens/profile/feedback_bottom_sheet.dart';
import 'package:typie/widgets/activity_grid.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/svg_image.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

const _cardRadius = 12.0;
const _sectionGap = 16.0;

@RoutePage()
class ProfileScreen extends StatelessWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      child: GraphQLOperation(
        initialBackgroundColor: context.colors.surfaceSubtle,
        operation: GProfileScreen_QueryReq(),
        builder: (context, client, data) => _Content(data: data, client: client),
      ),
    );
  }
}

class _Content extends HookWidget {
  const _Content({required this.data, required this.client});

  final GProfileScreen_QueryData data;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final me = data.me!;
    final scrollController = useScrollController();

    return Stack(
      children: [
        SingleChildScrollView(
          controller: scrollController,
          physics: const AlwaysScrollableScrollPhysics(),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Gap(OverlayHeading.contentTopSpacing),
              const Padding(
                padding: Pad(horizontal: 20, top: 8, bottom: 4),
                child: Text('프로필', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
              ),
              _ProfileHero(me: me),
              _ProfileActivitySection(data: data),
              _ProfileAccountSection(me: me),
              _ProfileSupportSection(me: me, client: client),
              const Gap(140),
            ],
          ),
        ),
        _Heading(me: me, scrollController: scrollController),
      ],
    );
  }
}

class _Heading extends StatelessWidget {
  const _Heading({required this.me, required this.scrollController});

  final GProfileScreen_QueryData_me me;
  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    return OverlayHeading(
      title: '프로필',
      scrollController: scrollController,
      leading: Tappable(
        onTap: () async {
          await context.router.push(const UpdateProfileRoute());
        },
        child: Tappable.scale(
          scale: 0.95,
          child: Container(
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              boxShadow: [
                BoxShadow(
                  color: context.colors.shadowDefault.withValues(alpha: 0.08),
                  blurRadius: 4,
                  offset: const Offset(0, 1),
                ),
              ],
            ),
            child: ClipOval(
              child: CachedNetworkImage(
                imageUrl: _avatarUrl(context, me.avatar.url, 36),
                width: 36,
                height: 36,
                fit: BoxFit.cover,
                fadeInDuration: const Duration(milliseconds: 150),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ProfileHero extends StatelessWidget {
  const _ProfileHero({required this.me});

  final GProfileScreen_QueryData_me me;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const Pad(horizontal: 20, top: _sectionGap),
      child: Tappable(
        onTap: () async {
          await context.router.push(const UpdateProfileRoute());
        },
        child: DecoratedBox(
          decoration: _cardDecoration(context),
          child: Tappable.scale(
            child: Padding(
              padding: const Pad(all: 18),
              child: Row(
                spacing: 16,
                children: [
                  ClipOval(
                    child: CachedNetworkImage(
                      imageUrl: _avatarUrl(context, me.avatar.url, 72),
                      width: 72,
                      height: 72,
                      fit: BoxFit.cover,
                      fadeInDuration: const Duration(milliseconds: 150),
                    ),
                  ),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          me.name,
                          style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const Gap(4),
                        Text(
                          me.email,
                          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ),
                  ),
                  Icon(LucideLightIcons.chevron_right, size: 16, color: context.colors.textFaint),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ProfileActivitySection extends StatelessWidget {
  const _ProfileActivitySection({required this.data});

  final GProfileScreen_QueryData data;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const Pad(horizontal: 20, top: _sectionGap),
      child: Container(
        decoration: _cardDecoration(context),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const Pad(horizontal: 16, top: 16, bottom: 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('나의 글쓰기 활동', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                  const Gap(3),
                  Text(
                    '지난 1년 동안의 기록이에요',
                    style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                  ),
                ],
              ),
            ),
            ActivityGrid(
              changes: [
                for (final change
                    in data.me?.characterCountChanges.whereType<GProfileScreen_QueryData_me_characterCountChanges>() ??
                        const <GProfileScreen_QueryData_me_characterCountChanges>[])
                  ActivityGridChange(date: change.date, additions: change.additions),
              ],
            ),
            HorizontalDivider(color: context.colors.borderSubtle),
            _ListRow(
              icon: LucideLightIcons.bar_chart_3,
              label: '통계',
              onTap: () async {
                await context.router.push(const StatsRoute());
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _ProfileAccountSection extends StatelessWidget {
  const _ProfileAccountSection({required this.me});

  final GProfileScreen_QueryData_me me;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const Pad(horizontal: 20, top: _sectionGap),
          child: Container(
            decoration: _cardDecoration(context),
            clipBehavior: Clip.antiAlias,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Tappable(
                  padding: const Pad(all: 16),
                  onTap: () async {
                    if (me.subscription == null) {
                      await context.router.push(const EnrollPlanRoute());
                    } else {
                      await context.router.push(const CurrentPlanRoute());
                    }
                  },
                  child: Tappable.scale(
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      spacing: 12,
                      children: [
                        Icon(LucideLightIcons.credit_card, size: 18, color: context.colors.textSubtle),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Text('현재 이용권', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                              const Gap(2),
                              Text(
                                me.subscription == null ? '타이피 BASIC ACCESS' : me.subscription!.plan.name,
                                style: TextStyle(
                                  fontSize: 14,
                                  fontWeight: FontWeight.w500,
                                  color: context.colors.textFaint,
                                ),
                              ),
                            ],
                          ),
                        ),
                        Row(
                          mainAxisSize: MainAxisSize.min,
                          spacing: 2,
                          children: [
                            Text(
                              me.subscription == null ? '구매하기' : '이용권 정보',
                              style: TextStyle(
                                fontSize: 14,
                                fontWeight: FontWeight.w500,
                                color: context.colors.textFaint,
                              ),
                            ),
                            Icon(LucideLightIcons.chevron_right, size: 14, color: context.colors.textFaint),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
                HorizontalDivider(color: context.colors.borderSubtle),
                _ListRow(
                  icon: LucideLightIcons.gift,
                  label: '초대',
                  onTap: () async {
                    await context.router.push(const ReferralRoute());
                  },
                ),
              ],
            ),
          ),
        ),
        const Gap(_sectionGap),
        Padding(
          padding: const Pad(horizontal: 20),
          child: Container(
            decoration: _cardDecoration(context),
            clipBehavior: Clip.antiAlias,
            child: _ListRow(
              icon: LucideLightIcons.settings,
              label: '설정',
              onTap: () async {
                await context.router.push(const SettingsRoute());
              },
            ),
          ),
        ),
      ],
    );
  }
}

class _ProfileSupportSection extends StatelessWidget {
  const _ProfileSupportSection({required this.me, required this.client});

  final GProfileScreen_QueryData_me me;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const _SectionLabel(text: '도움 및 링크', top: 24),
        Padding(
          padding: const Pad(horizontal: 20),
          child: IntrinsicHeight(
            child: Row(
              spacing: 16,
              children: [
                Expanded(
                  child: _QuickLinkCard(
                    icon: LucideLightIcons.headphones,
                    trailingIcon: LucideLightIcons.external_link,
                    label: '고객센터',
                    onTap: () async {
                      final url = Uri.parse('https://penxle.channel.io/home');
                      await launchUrl(url, mode: LaunchMode.externalApplication);
                    },
                  ),
                ),
                Expanded(
                  child: _QuickLinkCard(
                    icon: LucideLightIcons.message_square,
                    label: '의견 보내기',
                    onTap: () async {
                      await context.showBottomSheet(intercept: true, child: FeedbackBottomSheet(client: client));
                    },
                  ),
                ),
              ],
            ),
          ),
        ),
        const Gap(16),
        Padding(
          padding: const Pad(horizontal: 20),
          child: Container(
            decoration: _cardDecoration(context),
            clipBehavior: Clip.antiAlias,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                if (me.subscription != null) ...[
                  _ListRow(
                    iconWidget: const SvgImage('brands/discord', width: 20, height: 20),
                    label: '타이피 유저 커뮤니티',
                    trailingIcon: LucideLightIcons.external_link,
                    onTap: () async {
                      final url = Uri.parse('https://typie.link/community');
                      await launchUrl(url, mode: LaunchMode.externalApplication);
                    },
                  ),
                  HorizontalDivider(color: context.colors.borderSubtle),
                ],
                _ListRow(
                  iconWidget: const SvgImage('brands/x', width: 18, height: 18),
                  label: '타이피 공식 X',
                  trailingIcon: LucideLightIcons.external_link,
                  onTap: () async {
                    final url = Uri.parse('https://x.com/typieofficial');
                    await launchUrl(url, mode: LaunchMode.externalApplication);
                  },
                ),
                HorizontalDivider(color: context.colors.borderSubtle),
                _ListRow(
                  icon: LucideLightIcons.newspaper,
                  label: '업데이트 노트',
                  trailingIcon: LucideLightIcons.external_link,
                  onTap: () async {
                    final url = Uri.parse('https://typie.co/changelog');
                    await launchUrl(url, mode: LaunchMode.externalApplication);
                  },
                ),
              ],
            ),
          ),
        ),
      ],
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
      padding: Pad(horizontal: 20, top: top, bottom: 12),
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

class _QuickLinkCard extends StatelessWidget {
  const _QuickLinkCard({required this.icon, required this.label, required this.onTap, this.trailingIcon});

  final IconData icon;
  final String label;
  final Future<void> Function() onTap;
  final IconData? trailingIcon;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () async {
        await onTap();
      },
      child: DecoratedBox(
        decoration: _cardDecoration(context),
        child: Tappable.scale(
          child: Padding(
            padding: const Pad(all: 16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Icon(icon, size: 20, color: context.colors.textSubtle),
                    if (trailingIcon != null) Icon(trailingIcon, size: 16, color: context.colors.textFaint),
                  ],
                ),
                const Gap(18),
                Text(label, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ListRow extends StatelessWidget {
  const _ListRow({
    required this.label,
    required this.onTap,
    this.icon,
    this.iconWidget,
    this.trailingIcon = LucideLightIcons.chevron_right,
  }) : assert(icon != null || iconWidget != null);

  final String label;
  final Future<void> Function() onTap;
  final IconData? icon;
  final Widget? iconWidget;
  final IconData trailingIcon;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      padding: const Pad(all: 16),
      onTap: () async {
        await onTap();
      },
      child: Tappable.scale(
        child: Row(
          spacing: 10,
          children: [
            iconWidget ?? Icon(icon, size: 20, color: context.colors.textSubtle),
            Expanded(
              child: Text(label, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
            ),
            Icon(trailingIcon, size: 16, color: context.colors.textFaint),
          ],
        ),
      ),
    );
  }
}

BoxDecoration _cardDecoration(BuildContext context) =>
    BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(_cardRadius));

String _avatarUrl(BuildContext context, String url, double size) {
  final pixelSize = size * MediaQuery.devicePixelRatioOf(context);
  final roundedSize = math.pow(2, (math.log(pixelSize) / math.log(2)).ceil()).toInt();
  return '$url?s=$roundedSize&q=75';
}
