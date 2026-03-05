import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/profile/__generated__/profile_query.req.gql.dart';
import 'package:typie/screens/profile/activity_grid.dart';
import 'package:typie/screens/profile/feedback_bottom_sheet.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

@RoutePage()
class ProfileScreen extends StatelessWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: const Heading(title: '프로필', titleIcon: LucideLightIcons.circle_user_round),
      child: GraphQLOperation(
        operation: GProfileScreen_QueryReq(),
        builder: (context, client, data) {
          return SingleChildScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const Pad(all: 20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 16,
              children: [
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: context.colors.borderStrong),
                    borderRadius: BorderRadius.circular(8),
                    color: context.colors.surfaceDefault,
                  ),
                  padding: const Pad(horizontal: 16, vertical: 24),
                  child: Column(
                    spacing: 2,
                    children: [
                      Tappable(
                        onTap: () async {
                          await context.router.push(const UpdateProfileRoute());
                        },
                        child: ClipOval(
                          child: CachedNetworkImage(
                            imageUrl:
                                '${data.me!.avatar.url}?s=${pow(2, (log(80 * MediaQuery.devicePixelRatioOf(context)) / log(2)).ceil()).toInt()}&q=75',
                            width: 80,
                            height: 80,
                            fit: BoxFit.cover,
                            fadeInDuration: const Duration(milliseconds: 150),
                          ),
                        ),
                      ),
                      const Gap(8),
                      Tappable(
                        onTap: () async {
                          await context.router.push(const UpdateProfileRoute());
                        },
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          spacing: 4,
                          children: [
                            Flexible(
                              child: Text(
                                data.me!.name,
                                style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            Icon(LucideLightIcons.pencil, size: 14, color: context.colors.textFaint),
                          ],
                        ),
                      ),
                      Text(data.me!.email, style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
                    ],
                  ),
                ),
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: context.colors.borderStrong),
                    borderRadius: BorderRadius.circular(8),
                    color: context.colors.surfaceDefault,
                  ),
                  padding: const Pad(top: 16, bottom: 8),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Padding(
                        padding: Pad(horizontal: 16),
                        child: Text('나의 글쓰기 활동', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                      ),
                      ActivityGrid(characterCountChanges: data.me?.characterCountChanges.toList() ?? []),
                    ],
                  ),
                ),
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: context.colors.borderStrong),
                    borderRadius: BorderRadius.circular(8),
                    color: context.colors.surfaceDefault,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Tappable(
                        padding: const Pad(all: 16),
                        onTap: () async {
                          if (data.me!.subscription == null) {
                            await context.router.push(const EnrollPlanRoute());
                          } else {
                            await context.router.push(const CurrentPlanRoute());
                          }
                        },
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          spacing: 4,
                          children: [
                            Row(
                              children: [
                                Text('현재 이용권', style: TextStyle(color: context.colors.textFaint, fontSize: 14)),
                                const Spacer(),
                                if (data.me!.subscription == null) ...[
                                  Text('이용권 구매하기', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                                  Icon(LucideLightIcons.chevron_right, size: 14, color: context.colors.textSubtle),
                                ] else ...[
                                  Text('이용권 정보', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
                                  Icon(LucideLightIcons.chevron_right, size: 14, color: context.colors.textFaint),
                                ],
                              ],
                            ),
                            Text(
                              data.me!.subscription == null ? '타이피 BASIC ACCESS' : data.me!.subscription!.plan.name,
                              style: const TextStyle(fontWeight: FontWeight.w600),
                            ),
                          ],
                        ),
                      ),
                      HorizontalDivider(color: context.colors.borderDefault),
                      Tappable(
                        padding: const Pad(all: 16),
                        onTap: () async {
                          await context.router.push(const ReferralRoute());
                        },
                        child: const Row(
                          spacing: 8,
                          children: [
                            Icon(LucideLightIcons.gift, size: 20),
                            Expanded(
                              child: Text('초대', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                            ),
                            Icon(LucideLightIcons.chevron_right, size: 16),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
                Tappable(
                  onTap: () async {
                    await context.router.push(const SettingsRoute());
                  },
                  child: Container(
                    decoration: BoxDecoration(
                      border: Border.all(color: context.colors.borderStrong),
                      borderRadius: BorderRadius.circular(8),
                      color: context.colors.surfaceDefault,
                    ),
                    padding: const Pad(all: 16),
                    child: const Row(
                      spacing: 8,
                      children: [
                        Icon(LucideLightIcons.settings, size: 20),
                        Expanded(
                          child: Text('설정', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                        ),
                        Icon(LucideLightIcons.chevron_right, size: 16),
                      ],
                    ),
                  ),
                ),
                Padding(
                  padding: const Pad(top: 8),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    spacing: 12,
                    children: [
                      Text(
                        '도움 및 외부 링크',
                        style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                      ),
                      IntrinsicHeight(
                        child: Row(
                          spacing: 16,
                          children: [
                            Expanded(
                              child: Container(
                                decoration: BoxDecoration(
                                  border: Border.all(color: context.colors.borderStrong),
                                  borderRadius: BorderRadius.circular(8),
                                  color: context.colors.surfaceDefault,
                                ),
                                padding: const Pad(all: 16),
                                child: Tappable(
                                  onTap: () async {
                                    final url = Uri.parse('https://penxle.channel.io/home');
                                    await launchUrl(url, mode: LaunchMode.externalApplication);
                                  },
                                  child: const Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                    spacing: 12,
                                    children: [
                                      Row(
                                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                        children: [
                                          Icon(LucideLightIcons.headphones, size: 20),
                                          Icon(LucideLightIcons.external_link, size: 16),
                                        ],
                                      ),
                                      Text('고객센터', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                                    ],
                                  ),
                                ),
                              ),
                            ),
                            Expanded(
                              child: Container(
                                decoration: BoxDecoration(
                                  border: Border.all(color: context.colors.borderStrong),
                                  borderRadius: BorderRadius.circular(8),
                                  color: context.colors.surfaceDefault,
                                ),
                                padding: const Pad(all: 16),
                                child: Tappable(
                                  onTap: () async {
                                    await context.showBottomSheet(
                                      intercept: true,
                                      resizeToAvoidBottomInset: true,
                                      child: FeedbackBottomSheet(client: client),
                                    );
                                  },
                                  child: const Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    spacing: 12,
                                    children: [
                                      Icon(LucideLightIcons.message_square, size: 20),
                                      Text('의견 보내기', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                                    ],
                                  ),
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                      Container(
                        decoration: BoxDecoration(
                          border: Border.all(color: context.colors.borderStrong),
                          borderRadius: BorderRadius.circular(8),
                          color: context.colors.surfaceDefault,
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            if (data.me!.subscription != null) ...[
                              Tappable(
                                padding: const Pad(all: 16),
                                onTap: () async {
                                  final url = Uri.parse('https://typie.link/community');
                                  await launchUrl(url, mode: LaunchMode.externalApplication);
                                },
                                child: const Row(
                                  spacing: 8,
                                  children: [
                                    Icon(LucideLightIcons.users, size: 20),
                                    Expanded(
                                      child: Text(
                                        '타이피 유저 커뮤니티',
                                        style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                                      ),
                                    ),
                                    Icon(LucideLightIcons.external_link, size: 16),
                                  ],
                                ),
                              ),
                              HorizontalDivider(color: context.colors.borderDefault),
                            ],
                            Tappable(
                              padding: const Pad(all: 16),
                              onTap: () async {
                                final url = Uri.parse('https://x.com/typieofficial');
                                await launchUrl(url, mode: LaunchMode.externalApplication);
                              },
                              child: const Row(
                                spacing: 8,
                                children: [
                                  Icon(LucideLightIcons.twitter, size: 20),
                                  Expanded(
                                    child: Text(
                                      '타이피 공식 트위터',
                                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                                    ),
                                  ),
                                  Icon(LucideLightIcons.external_link, size: 16),
                                ],
                              ),
                            ),
                            HorizontalDivider(color: context.colors.borderDefault),
                            Tappable(
                              padding: const Pad(all: 16),
                              onTap: () async {
                                final url = Uri.parse('https://typie.co/changelog');
                                await launchUrl(url, mode: LaunchMode.externalApplication);
                              },
                              child: const Row(
                                spacing: 8,
                                children: [
                                  Icon(LucideLightIcons.newspaper, size: 20),
                                  Expanded(
                                    child: Text('업데이트 노트', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                                  ),
                                  Icon(LucideLightIcons.external_link, size: 16),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}
