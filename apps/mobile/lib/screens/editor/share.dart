import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/switch.dart';

class ShareBottomSheet extends StatelessWidget {
  const ShareBottomSheet({super.key});

  @override
  Widget build(BuildContext context) {
    return AppFullBottomSheet(
      title: '이 포스트 공유하기',
      child: HookForm(
        builder: (context, form) {
          return const Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 40,
            children: [
              _Section(
                title: '포스트 조회 권한',
                children: [
                  _Option(
                    icon: LucideLightIcons.blend,
                    label: '공개 범위',
                    trailing: HookFormSelect(
                      name: 'visibility',
                      initialValue: GEntityVisibility.UNLISTED,
                      items: [
                        HookFormSelectItem(
                          icon: LucideLightIcons.link,
                          label: '링크가 있는 사람',
                          description: '링크가 있는 누구나 볼 수 있어요.',
                          value: GEntityVisibility.UNLISTED,
                        ),
                        HookFormSelectItem(
                          icon: LucideLightIcons.lock,
                          label: '비공개',
                          description: '나만 볼 수 있어요.',
                          value: GEntityVisibility.PRIVATE,
                        ),
                      ],
                    ),
                  ),
                  _Option(
                    icon: LucideLightIcons.lock_keyhole,
                    label: '비밀번호 보호',
                    trailing: HookFormSwitch(name: 'hasPassword'),
                  ),
                  _Option(
                    icon: LucideLightIcons.id_card,
                    label: '연령 제한',
                    trailing: HookFormSelect(
                      name: 'contentRating',
                      initialValue: GPostContentRating.ALL,
                      items: [
                        HookFormSelectItem(label: '없음', value: GPostContentRating.ALL),
                        HookFormSelectItem(label: '15세', value: GPostContentRating.R15),
                        HookFormSelectItem(label: '성인', value: GPostContentRating.R19),
                      ],
                    ),
                  ),
                ],
              ),
              _Section(
                title: '포스트 상호작용',
                children: [
                  _Option(
                    icon: LucideLightIcons.message_square,
                    label: '댓글',
                    trailing: HookFormSelect(
                      name: 'allowComment',
                      initialValue: true,
                      items: [
                        HookFormSelectItem(icon: LucideLightIcons.circle_user_round, label: '로그인한 이용자', value: true),
                        HookFormSelectItem(icon: LucideLightIcons.ban, label: '비허용', value: false),
                      ],
                    ),
                  ),
                  _Option(
                    icon: LucideLightIcons.smile,
                    label: '이모지 반응',
                    trailing: HookFormSelect(
                      name: 'allowReaction',
                      initialValue: true,
                      items: [
                        HookFormSelectItem(icon: LucideLightIcons.users_round, label: '누구나', value: true),
                        HookFormSelectItem(icon: LucideLightIcons.ban, label: '비허용', value: false),
                      ],
                    ),
                  ),
                ],
              ),
              _Section(
                title: '포스트 보호',
                children: [
                  _Option(
                    icon: LucideLightIcons.shield,
                    label: '내용 보호',
                    trailing: HookFormSwitch(name: 'protectContent'),
                  ),
                ],
              ),
            ],
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
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 16,
      children: [
        Text(title, style: const TextStyle(fontSize: 16, color: AppColors.gray_500)),
        ...children,
      ],
    );
  }
}

class _Option extends StatelessWidget {
  const _Option({required this.icon, required this.label, required this.trailing});

  final IconData icon;
  final String label;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 24,
      child: Row(
        children: [
          Icon(icon, size: 20, color: AppColors.gray_500),
          const Gap(8),
          Expanded(
            child: Text(label, style: const TextStyle(fontSize: 16, color: AppColors.gray_700)),
          ),
          trailing,
        ],
      ),
    );
  }
}
