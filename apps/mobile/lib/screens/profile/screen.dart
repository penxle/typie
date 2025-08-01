import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/profile/__generated__/profile_query.req.gql.dart';
import 'package:typie/screens/profile/activity_grid.dart';
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
                  padding: const Pad(all: 16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    spacing: 12,
                    children: [
                      const Text('나의 글쓰기 활동', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                      ActivityGrid(characterCountChanges: data.me?.characterCountChanges.toList() ?? []),
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
                      border: Border.all(color: context.colors.borderStrong),
                      borderRadius: BorderRadius.circular(8),
                      color: context.colors.surfaceDefault,
                    ),
                    padding: const Pad(all: 16),
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
                            border: Border.all(color: context.colors.borderStrong),
                            borderRadius: BorderRadius.circular(8),
                            color: context.colors.surfaceDefault,
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
                Tappable(
                  onTap: () async {
                    final packageInfo = await PackageInfo.fromPlatform();
                    final deviceInfo = await DeviceInfoPlugin().deviceInfo;

                    final (os, model) = switch (deviceInfo) {
                      IosDeviceInfo() => (
                        '${deviceInfo.systemName} ${deviceInfo.systemVersion}',
                        '${deviceInfo.modelName} (${deviceInfo.model})',
                      ),
                      AndroidDeviceInfo() => (
                        '${deviceInfo.version.baseOS} ${deviceInfo.version.release}',
                        '${deviceInfo.manufacturer} ${deviceInfo.model} (${deviceInfo.brand})',
                      ),
                      _ => throw UnimplementedError(),
                    };

                    final encodedSubject = Uri.encodeComponent('[타이피] 앱 개선 의견');
                    final encodedBody = Uri.encodeComponent(
                      '-----\n'
                      '로그인 정보: ${data.me!.id}\n'
                      '버전 정보: ${packageInfo.version} (${packageInfo.buildNumber})\n'
                      '디바이스 정보: $model - $os\n'
                      '-----\n'
                      '타이피 앱 및 서비스에 대한 개선 의견을 자유롭게 적어주세요. 모든 의견은 타이피 팀에서 꼼꼼히 확인하고 있어요.\n'
                      '-----\n\n',
                    );

                    final uri = Uri.parse('mailto:hello@penxle.io?subject=$encodedSubject&body=$encodedBody');

                    if (await canLaunchUrl(uri)) {
                      await launchUrl(uri);
                    } else {
                      if (context.mounted) {
                        context.toast(ToastType.error, '메일 앱을 먼저 설치해주세요.');
                      }
                    }
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
                        Icon(LucideLightIcons.message_circle, size: 20),
                        Expanded(
                          child: Text('앱 개선 의견 보내기', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500)),
                        ),
                        Icon(LucideLightIcons.chevron_right, size: 16),
                      ],
                    ),
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
