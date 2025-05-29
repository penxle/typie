import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/settings/__generated__/screen.req.gql.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class SettingsScreen extends HookWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();

    return Screen(
      heading: const Heading(title: '설정'),
      child: GraphQLOperation(
        operation: GSettingsScreen_QueryReq(),
        builder: (context, client, data) {
          return SingleChildScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: Pad(all: 20, bottom: MediaQuery.paddingOf(context).bottom),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 24,
              children: [
                _Section(
                  title: '계정 설정',
                  children: [
                    _Item(
                      label: '이메일 변경',
                      onTap: () async {
                        await context.router.push(const UpdateEmailRoute());
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '프로필 변경',
                      onTap: () async {
                        await context.router.push(const UpdateProfileRoute());
                      },
                    ),
                  ],
                ),
                if (data.me!.plan != null)
                  _Section(
                    title: '사이트 설정',
                    children: [
                      _Item(
                        label: '사이트 주소 변경',
                        onTap: () async {
                          await context.router.push(const UpdateSiteSlugRoute());
                        },
                      ),
                    ],
                  ),
                _Section(
                  title: '이벤트 알림 설정',
                  children: [_Item(label: '이벤트 및 타이피 소식 받아보기', onTap: () {})],
                ),
                _Section(
                  title: '서비스 정보',
                  children: [
                    _Item(label: '이용약관', onTap: () {}),
                    const _Divider(),
                    _Item(label: '사업자 정보', onTap: () {}),
                    const _Divider(),
                    _Item(label: '오픈소스 라이센스', onTap: () {}),
                    const _Divider(),
                    _Item(label: '버전 정보', onTap: () {}),
                  ],
                ),
                _Section(
                  title: '기타',
                  children: [
                    _Item(
                      label: '로그아웃',
                      onTap: () async {
                        await context.showModal(
                          child: ConfirmModal(
                            title: '로그아웃',
                            message: '정말 로그아웃하시겠어요?',
                            confirmText: '로그아웃',
                            onConfirm: () async {
                              await auth.logout();
                            },
                          ),
                        );
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '회원 탈퇴',
                      onTap: () async {
                        await context.router.push(const DeleteUserRoute());
                      },
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

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 8,
      children: [
        Text(
          title,
          style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: AppColors.gray_500),
        ),
        Container(
          decoration: BoxDecoration(
            border: Border.all(color: AppColors.gray_950),
            borderRadius: BorderRadius.circular(8),
            color: AppColors.white,
          ),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: children),
        ),
      ],
    );
  }
}

class _Divider extends StatelessWidget {
  const _Divider();

  @override
  Widget build(BuildContext context) {
    return const HorizontalDivider(color: AppColors.gray_200);
  }
}

class _Item extends StatelessWidget {
  const _Item({required this.onTap, required this.label});

  final void Function() onTap;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      padding: const Pad(all: 16),
      child: Row(
        children: [
          Expanded(child: Text(label, style: const TextStyle(fontSize: 16))),
          const Icon(LucideLightIcons.chevron_right, size: 16),
        ],
      ),
    );
  }
}
