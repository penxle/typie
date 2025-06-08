import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:share_plus/share_plus.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/env.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/share_query.req.gql.dart';
import 'package:typie/screens/editor/__generated__/update_post_option_mutation.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/switch.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/tappable.dart';

class ShareBottomSheet extends StatelessWidget {
  const ShareBottomSheet({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    return AppFullBottomSheet(
      title: '이 포스트 공유하기',
      padding: null,
      child: GraphQLOperation(
        operation: GEditorScreen_Share_QueryReq((b) => b..vars.slug = slug),
        builder: (context, client, data) {
          return HookForm(
            submitMode: HookFormSubmitMode.onChange,
            onSubmit: (form) async {
              await client.request(
                GEditorScreen_Share_UpdatePostOption_MutationReq(
                  (b) => b
                    ..vars.input.postId = data.post.id
                    ..vars.input.visibility = form.data['visibility'] as GEntityVisibility
                    ..vars.input.contentRating = form.data['contentRating'] as GPostContentRating
                    ..vars.input.password = form.data['hasPassword'] as bool ? form.data['password'] as String? : null
                    ..vars.input.allowComment = form.data['allowComment'] as bool
                    ..vars.input.allowReaction = form.data['allowReaction'] as bool
                    ..vars.input.protectContent = form.data['protectContent'] as bool,
                ),
              );
            },
            builder: (context, form) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    child: Padding(
                      padding: const Pad(all: 20),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        spacing: 32,
                        children: [
                          _Section(
                            title: '포스트 조회 권한',
                            children: [
                              _Option(
                                icon: LucideLightIcons.blend,
                                label: '공개 범위',
                                trailing: HookFormSelect(
                                  name: 'visibility',
                                  initialValue: data.post.entity.visibility,
                                  items: const [
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
                                icon: LucideLightIcons.id_card,
                                label: '연령 제한',
                                trailing: HookFormSelect(
                                  name: 'contentRating',
                                  initialValue: data.post.contentRating,
                                  items: const [
                                    HookFormSelectItem(label: '없음', value: GPostContentRating.ALL),
                                    HookFormSelectItem(label: '15세', value: GPostContentRating.R15),
                                    HookFormSelectItem(label: '성인', value: GPostContentRating.R19),
                                  ],
                                ),
                              ),
                              _Option(
                                icon: LucideLightIcons.lock_keyhole,
                                label: '비밀번호 보호',
                                trailing: HookFormSwitch(name: 'hasPassword', initialValue: data.post.password != null),
                              ),
                              if (form.data['hasPassword'] as bool? ?? false)
                                HookFormTextField(
                                  name: 'password',
                                  label: '비밀번호',
                                  placeholder: '비밀번호를 입력해주세요.',
                                  keyboardType: TextInputType.visiblePassword,
                                  initialValue: data.post.password,
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
                                  initialValue: data.post.allowComment,
                                  items: const [
                                    HookFormSelectItem(
                                      icon: LucideLightIcons.circle_user_round,
                                      label: '로그인한 이용자',
                                      value: true,
                                    ),
                                    HookFormSelectItem(icon: LucideLightIcons.ban, label: '비허용', value: false),
                                  ],
                                ),
                              ),
                              _Option(
                                icon: LucideLightIcons.smile,
                                label: '이모지 반응',
                                trailing: HookFormSelect(
                                  name: 'allowReaction',
                                  initialValue: data.post.allowReaction,
                                  items: const [
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
                                trailing: HookFormSwitch(
                                  name: 'protectContent',
                                  initialValue: data.post.protectContent,
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ),
                  Tappable(
                    onTap: () async {
                      final baseUrl = Env.usersiteUrl.replaceAll('*.', '');
                      final url = Uri.parse('$baseUrl/${data.post.entity.permalink}');
                      await SharePlus.instance.share(ShareParams(title: data.post.title, uri: url));
                    },
                    child: Container(
                      alignment: Alignment.center,
                      decoration: const BoxDecoration(color: AppColors.gray_950),
                      padding: Pad(vertical: 16, bottom: MediaQuery.viewPaddingOf(context).bottom),
                      child: const Text(
                        '공유하기',
                        style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.white),
                      ),
                    ),
                  ),
                ],
              );
            },
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
        Text(
          title,
          style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.gray_700),
        ),
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
          Icon(icon, size: 20, color: AppColors.gray_700),
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
