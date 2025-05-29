import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/setting/__generated__/screen.req.gql.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class SettingScreen extends HookWidget {
  const SettingScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();

    return Screen(
      heading: const Heading(title: '설정'),
      padding: const Pad(all: 20),
      child: GraphQLOperation(
        operation: GSettingScreen_QueryReq(),
        builder: (context, client, data) {
          return SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 24,
              children: [
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 8,
                  children: [
                    const Text(
                      '계정 설정',
                      style: TextStyle(fontSize: 13, color: AppColors.gray_600, fontWeight: FontWeight.w500),
                    ),
                    Container(
                      decoration: BoxDecoration(
                        border: Border.all(color: AppColors.gray_950),
                        borderRadius: BorderRadius.circular(8),
                        color: AppColors.white,
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          _Setting(
                            onTap: () async {
                              await context.router.push(const UpdateEmailRoute());
                            },
                            label: '이메일 변경',
                          ),
                          const HorizontalDivider(color: AppColors.gray_600),
                          _Setting(onTap: () {}, label: '본인 인증'),
                        ],
                      ),
                    ),
                  ],
                ),
                if (data.me!.plan != null)
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    spacing: 8,
                    children: [
                      const Text(
                        '사이트 설정',
                        style: TextStyle(fontSize: 13, color: AppColors.gray_600, fontWeight: FontWeight.w500),
                      ),
                      Container(
                        decoration: BoxDecoration(
                          border: Border.all(color: AppColors.gray_950),
                          borderRadius: BorderRadius.circular(8),
                          color: AppColors.white,
                        ),
                        child: _Setting(onTap: () {}, label: '사이트 주소 변경'),
                      ),
                    ],
                  ),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 8,
                  children: [
                    const Text(
                      '이벤트 알림 설정',
                      style: TextStyle(fontSize: 13, color: AppColors.gray_600, fontWeight: FontWeight.w500),
                    ),
                    Container(
                      decoration: BoxDecoration(
                        border: Border.all(color: AppColors.gray_950),
                        borderRadius: BorderRadius.circular(8),
                        color: AppColors.white,
                      ),
                      padding: const Pad(horizontal: 16, vertical: 18),
                      child: const Row(
                        spacing: 6,
                        children: [
                          Expanded(
                            child: Text('이벤트 및 타이피 소식 받아보기', style: TextStyle(fontWeight: FontWeight.w500)),
                          ),
                          Icon(LucideLightIcons.chevron_right, size: 16, fontWeight: FontWeight.bold),
                        ],
                      ),
                    ),
                  ],
                ),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 8,
                  children: [
                    const Text(
                      '서비스 정보',
                      style: TextStyle(fontSize: 13, color: AppColors.gray_600, fontWeight: FontWeight.w500),
                    ),
                    Container(
                      decoration: BoxDecoration(
                        border: Border.all(color: AppColors.gray_950),
                        borderRadius: BorderRadius.circular(8),
                        color: AppColors.white,
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          _Setting(onTap: () {}, label: '이용약관'),
                          const HorizontalDivider(color: AppColors.gray_600),
                          _Setting(onTap: () {}, label: '사업자 정보'),
                          const HorizontalDivider(color: AppColors.gray_600),
                          _Setting(onTap: () {}, label: '오픈소스 라이센스'),
                          const HorizontalDivider(color: AppColors.gray_600),
                          _Setting(onTap: () {}, label: '버전 정보'),
                        ],
                      ),
                    ),
                  ],
                ),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 8,
                  children: [
                    const Text(
                      '기타',
                      style: TextStyle(fontSize: 13, color: AppColors.gray_600, fontWeight: FontWeight.w500),
                    ),
                    Container(
                      decoration: BoxDecoration(
                        border: Border.all(color: AppColors.gray_950),
                        borderRadius: BorderRadius.circular(8),
                        color: AppColors.white,
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          _Setting(
                            onTap: () async {
                              await auth.logout();
                            },
                            label: '로그아웃',
                          ),
                          const HorizontalDivider(color: AppColors.gray_600),
                          _Setting(onTap: () {}, label: '회원탈퇴'),
                        ],
                      ),
                    ),
                  ],
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _Setting extends StatelessWidget {
  const _Setting({required this.onTap, required this.label});

  final void Function() onTap;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      padding: const Pad(horizontal: 16, vertical: 18),
      child: Row(
        spacing: 6,
        children: [
          Expanded(
            child: Text(label, style: const TextStyle(fontWeight: FontWeight.w500)),
          ),
          const Icon(LucideLightIcons.chevron_right, size: 16, fontWeight: FontWeight.bold),
        ],
      ),
    );
  }
}
