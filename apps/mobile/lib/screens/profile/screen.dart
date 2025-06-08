import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/profile/__generated__/profile_query.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
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
          return Container(
            padding: const Pad(all: 20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 16,
              children: [
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: AppColors.gray_950),
                    borderRadius: BorderRadius.circular(8),
                    color: AppColors.white,
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
                            const Icon(LucideLightIcons.pencil, size: 14, color: AppColors.gray_500),
                          ],
                        ),
                      ),
                      Text(data.me!.email, style: const TextStyle(fontSize: 14, color: AppColors.gray_500)),
                    ],
                  ),
                ),
                Tappable(
                  onTap: () async {
                    if (data.me!.subscription == null) {
                      await context.router.push(const EnrollPlanRoute());
                    } else {
                      await context.router.push(const CurrentPlanRoute());
                    }
                  },
                  child: Container(
                    decoration: BoxDecoration(
                      border: Border.all(color: AppColors.gray_950),
                      borderRadius: BorderRadius.circular(8),
                      color: AppColors.white,
                    ),
                    padding: const Pad(all: 16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      spacing: 4,
                      children: [
                        Row(
                          children: [
                            const Text('현재 이용권', style: TextStyle(color: AppColors.gray_500, fontSize: 14)),
                            const Spacer(),
                            if (data.me!.subscription == null) ...[
                              const Text('이용권 구매하기', style: TextStyle(fontSize: 14, color: AppColors.gray_700)),
                              const Icon(LucideLightIcons.chevron_right, size: 14, color: AppColors.gray_700),
                            ] else ...[
                              const Text('이용권 정보', style: TextStyle(fontSize: 14, color: AppColors.gray_500)),
                              const Icon(LucideLightIcons.chevron_right, size: 14, color: AppColors.gray_500),
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
                ),
                Tappable(
                  onTap: () async {
                    await context.router.push(const SettingsRoute());
                  },
                  child: Container(
                    decoration: BoxDecoration(
                      border: Border.all(color: AppColors.gray_950),
                      borderRadius: BorderRadius.circular(8),
                      color: AppColors.white,
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
                IntrinsicHeight(
                  child: Row(
                    spacing: 16,
                    children: [
                      Expanded(
                        child: Container(
                          decoration: BoxDecoration(
                            border: Border.all(color: AppColors.gray_950),
                            borderRadius: BorderRadius.circular(8),
                            color: AppColors.white,
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
                                Icon(LucideLightIcons.headphones, size: 20),
                                Text('고객센터', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                              ],
                            ),
                          ),
                        ),
                      ),
                      Expanded(
                        child: Container(
                          decoration: BoxDecoration(
                            border: Border.all(color: AppColors.gray_950),
                            borderRadius: BorderRadius.circular(8),
                            color: AppColors.white,
                          ),
                          padding: const Pad(all: 16),
                          child: Tappable(
                            onTap: () async {
                              final url = Uri.parse('https://x.com/typieofficial');
                              await launchUrl(url, mode: LaunchMode.externalApplication);
                            },
                            child: const Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              spacing: 12,
                              children: [
                                Icon(LucideLightIcons.twitter, size: 20),
                                Text('타이피\n공식 트위터', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                              ],
                            ),
                          ),
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
