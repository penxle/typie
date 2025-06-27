import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:share_plus/share_plus.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/env.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/__generated__/share_folder_query.data.gql.dart';
import 'package:typie/modals/__generated__/share_folder_query.req.gql.dart';
import 'package:typie/modals/__generated__/share_post_query.req.gql.dart';
import 'package:typie/modals/__generated__/update_folder_option_mutation.req.gql.dart';
import 'package:typie/modals/__generated__/update_post_option_mutation.req.gql.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/switch.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/tappable.dart';

class SharePostBottomSheet extends HookWidget {
  const SharePostBottomSheet({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();

    return AppFullBottomSheet(
      title: '이 포스트 공유하기',
      child: GraphQLOperation(
        operation: GSharePost_QueryReq((b) => b..vars.slug = slug),
        builder: (context, client, data) {
          return HookForm(
            submitMode: HookFormSubmitMode.onChange,
            onSubmit: (form) async {
              final visibility = form.data['visibility'] as GEntityVisibility;
              final contentRating = form.data['contentRating'] as GPostContentRating;
              final hasPassword = form.data['hasPassword'] as bool;
              final allowComment = form.data['allowComment'] as bool;
              final allowReaction = form.data['allowReaction'] as bool;
              final protectContent = form.data['protectContent'] as bool;

              await client.request(
                GSharePost_UpdatePostOption_MutationReq(
                  (b) => b
                    ..vars.input.postId = data.post.id
                    ..vars.input.visibility = visibility
                    ..vars.input.contentRating = contentRating
                    ..vars.input.password = hasPassword ? form.data['password'] as String? : null
                    ..vars.input.allowComment = allowComment
                    ..vars.input.allowReaction = allowReaction
                    ..vars.input.protectContent = protectContent,
                ),
              );

              unawaited(
                mixpanel.track(
                  'update_post_option',
                  properties: {
                    'visibility': visibility.name,
                    'hasPassword': hasPassword,
                    'contentRating': contentRating.name,
                    'allowComment': allowComment,
                    'allowReaction': allowReaction,
                    'protectContent': protectContent,
                  },
                ),
              );
            },
            builder: (context, form) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
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
                              trailing: HookFormSwitch(name: 'protectContent', initialValue: data.post.protectContent),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                  Builder(
                    builder: (context) {
                      return Tappable(
                        onTap: () async {
                          final baseUrl = Env.usersiteUrl.replaceAll('*.', '');
                          final url = Uri.parse('$baseUrl/${data.post.entity.permalink}');

                          final box = context.findRenderObject() as RenderBox?;

                          try {
                            await SharePlus.instance.share(
                              ShareParams(
                                title: data.post.title,
                                uri: url,
                                sharePositionOrigin: box!.localToGlobal(Offset.zero) & box.size,
                              ),
                            );

                            unawaited(mixpanel.track('copy_post_share_url'));
                          } catch (_) {
                            // pass
                          }
                        },
                        child: Container(
                          decoration: BoxDecoration(
                            color: context.colors.surfaceToast,
                            borderRadius: BorderRadius.circular(8),
                          ),
                          padding: const Pad(vertical: 16),
                          child: Text(
                            '공유하기',
                            style: TextStyle(
                              fontSize: 16,
                              fontWeight: FontWeight.w700,
                              color: context.colors.textOnToast,
                            ),
                            textAlign: TextAlign.center,
                          ),
                        ),
                      );
                    },
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

class ShareFolderBottomSheet extends HookWidget {
  const ShareFolderBottomSheet({required this.entityId, super.key});

  final String entityId;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();

    return AppFullBottomSheet(
      title: '폴더 공유하기',
      child: GraphQLOperation(
        operation: GShareFolder_QueryReq((b) => b..vars.entityId = entityId),
        builder: (context, client, data) {
          return HookForm(
            submitMode: HookFormSubmitMode.onChange,
            onSubmit: (form) async {
              final visibility = form.data['visibility'] as GEntityVisibility;

              await client.request(
                GShareFolder_UpdateFolderOption_MutationReq(
                  (b) => b
                    ..vars.input.folderId = (data.entity.node as GShareFolder_QueryData_entity_node__asFolder).id
                    ..vars.input.visibility = visibility,
                ),
              );

              unawaited(mixpanel.track('update_folder_option', properties: {'visibility': visibility.name}));
            },
            builder: (context, form) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    child: _Section(
                      title: '폴더 조회 권한',
                      children: [
                        _Option(
                          icon: LucideLightIcons.blend,
                          label: '공개 범위',
                          trailing: HookFormSelect(
                            name: 'visibility',
                            initialValue: data.entity.visibility,
                            items: const [
                              HookFormSelectItem(
                                icon: LucideLightIcons.lock,
                                label: '비공개',
                                value: GEntityVisibility.PRIVATE,
                              ),
                              HookFormSelectItem(
                                icon: LucideLightIcons.link,
                                label: '링크가 있는 사람',
                                value: GEntityVisibility.UNLISTED,
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                  Builder(
                    builder: (context) => Tappable(
                      onTap: () async {
                        final baseUrl = Env.usersiteUrl.replaceAll('*.', '');
                        final url = Uri.parse('$baseUrl/${data.entity.permalink}');

                        final box = context.findRenderObject() as RenderBox?;

                        try {
                          await SharePlus.instance.share(
                            ShareParams(
                              title: data.entity.node.when(
                                folder: (folder) => folder.name,
                                orElse: () => throw UnimplementedError(),
                              ),
                              uri: url,
                              sharePositionOrigin: box!.localToGlobal(Offset.zero) & box.size,
                            ),
                          );

                          unawaited(mixpanel.track('copy_folder_share_url'));
                        } catch (_) {
                          // pass
                        }
                      },
                      child: Container(
                        decoration: BoxDecoration(
                          color: context.colors.surfaceToast,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        padding: const Pad(vertical: 16),
                        child: Text(
                          '공유하기',
                          style: TextStyle(
                            fontSize: 16,
                            fontWeight: FontWeight.w700,
                            color: context.colors.textOnToast,
                          ),
                          textAlign: TextAlign.center,
                        ),
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
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
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
          Icon(icon, size: 20, color: context.colors.textSubtle),
          const Gap(8),
          Expanded(
            child: Text(label, style: TextStyle(fontSize: 16, color: context.colors.textSubtle)),
          ),
          trailing,
        ],
      ),
    );
  }
}
