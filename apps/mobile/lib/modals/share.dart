import 'dart:async';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/built_value.dart' show EnumClass;
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:share_plus/share_plus.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/__generated__/share_entities_query.data.gql.dart';
import 'package:typie/modals/__generated__/share_entities_query.req.gql.dart';
import 'package:typie/modals/__generated__/update_folders_option_mutation.req.gql.dart';
import 'package:typie/modals/__generated__/update_posts_option_mutation.req.gql.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/switch.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/tappable.dart';

class ShareBottomSheet extends HookWidget {
  const ShareBottomSheet({required this.entityIds, super.key});

  final List<String> entityIds;

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      operation: GShareEntities_QueryReq((b) => b..vars.entityIds.addAll(entityIds)),
      builder: (context, client, data) {
        final entities = data.entities;
        final allFolders = entities.every((e) => e.type == GEntityType.FOLDER);
        final allPosts = entities.every((e) => e.type == GEntityType.POST);

        String title;
        if (allFolders) {
          title = entities.length == 1 ? '이 폴더 공유하기' : '폴더 ${entities.length}개 공유하기';
        } else if (allPosts) {
          title = entities.length == 1 ? '이 포스트 공유하기' : '포스트 ${entities.length}개 공유하기';
        } else {
          title = '공유하기';
        }

        return AppFullBottomSheet(
          title: title,
          child: Builder(
            builder: (context) {
              if (allFolders) {
                final folders = entities
                    .map((e) => e.node)
                    .whereType<GShareEntities_QueryData_entities_node__asFolder>()
                    .toList();
                return ShareFoldersContent(folders: folders, entities: entities.toList(), client: client);
              } else if (allPosts) {
                final posts = entities
                    .map((e) => e.node)
                    .whereType<GShareEntities_QueryData_entities_node__asPost>()
                    .toList();
                return SharePostsContent(posts: posts, entities: entities.toList(), client: client);
              }

              // NOTE: 혼합된 엔티티 타입은 지원하지 않음 - 이런 경우는 발생하지 않아야 함
              return const SizedBox.shrink();
            },
          ),
        );
      },
    );
  }
}

class SharePostsContent extends HookWidget {
  const SharePostsContent({required this.posts, required this.entities, required this.client, super.key});

  final List<GShareEntities_QueryData_entities_node__asPost> posts;
  final List<GShareEntities_QueryData_entities> entities;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final passwordController = useTextEditingController();
    final rotationController = useAnimationController(duration: const Duration(milliseconds: 500));
    final scaleController = useAnimationController(duration: const Duration(milliseconds: 250));
    final random = Random();

    final rotationAnimation = useMemoized(
      () =>
          Tween<double>(begin: 0, end: 1).animate(CurvedAnimation(parent: rotationController, curve: Curves.easeInOut)),
      [rotationController],
    );

    final scaleAnimation = useMemoized(
      () =>
          Tween<double>(begin: 1, end: 1.2).animate(CurvedAnimation(parent: scaleController, curve: Curves.easeInOut)),
      [scaleController],
    );

    final initialPassword = posts.length > 1 && posts.any((p) => p.password != posts.first.password)
        ? null
        : posts.first.password;

    useEffect(() {
      if (initialPassword != null) {
        passwordController.text = initialPassword;
      }
      return null;
    }, [initialPassword]);

    void generateRandomPassword() {
      const digits = '0123456789';
      final password = List.generate(4, (index) => digits[random.nextInt(digits.length)]).join();
      passwordController.text = password;

      unawaited(rotationController.forward(from: 0));
      unawaited(scaleController.forward(from: 0).then((_) => scaleController.reverse()));
    }

    return HookForm(
      submitMode: HookFormSubmitMode.onChange,
      onSubmit: (form) async {
        final dirtyData = form.getDirtyFieldsData();
        if (dirtyData.isEmpty) {
          return;
        }

        final builder = GSharePost_UpdatePostsOption_MutationReqBuilder();
        builder.vars.input.postIds.addAll(posts.map((p) => p.id));

        if (dirtyData.containsKey('visibility')) {
          builder.vars.input.visibility = Value.present(form.data['visibility'] as GEntityVisibility);
        }
        if (dirtyData.containsKey('contentRating')) {
          builder.vars.input.contentRating = Value.present(form.data['contentRating'] as GPostContentRating);
        }
        if (dirtyData.containsKey('hasPassword') || dirtyData.containsKey('password')) {
          builder.vars.input.password = Value.present(
            form.data['hasPassword'] as bool ? form.data['password'] as String? : null,
          );
        }
        if (dirtyData.containsKey('allowReaction')) {
          builder.vars.input.allowReaction = Value.present(form.data['allowReaction'] as bool);
        }
        if (dirtyData.containsKey('protectContent')) {
          builder.vars.input.protectContent = Value.present(form.data['protectContent'] as bool);
        }

        await client.request(builder.build());

        final trackProperties = <String, dynamic>{'count': posts.length};
        for (final entry in dirtyData.entries) {
          if (entry.value is EnumClass) {
            trackProperties[entry.key] = (entry.value as EnumClass).name;
          } else {
            trackProperties[entry.key] = entry.value;
          }
        }
        trackProperties['hasPassword'] = form.data['hasPassword'] as bool? ?? false;

        unawaited(mixpanel.track('update_post_option', properties: trackProperties));
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
                          initialValue: entities.first.visibility,
                          values: entities.map((e) => e.visibility).toList(),
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
                          initialValue: posts.first.contentRating,
                          values: posts.map((p) => p.contentRating).toList(),
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
                        trailing: HookFormSwitch(
                          name: 'hasPassword',
                          initialValue:
                              posts.map((p) => p.password != null).toSet().length == 1 && posts.first.password != null,
                          values: posts.map((p) => p.password != null).toList(),
                        ),
                      ),
                      if (form.data['hasPassword'] as bool? ?? false)
                        HookFormTextField(
                          name: 'password',
                          label: '비밀번호',
                          placeholder: '비밀번호를 입력해주세요.',
                          keyboardType: TextInputType.visiblePassword,
                          controller: passwordController,
                          initialValue: initialPassword,
                          suffix: GestureDetector(
                            onTap: generateRandomPassword,
                            behavior: HitTestBehavior.opaque,
                            child: Padding(
                              padding: const EdgeInsets.all(4),
                              child: RotationTransition(
                                turns: rotationAnimation,
                                child: ScaleTransition(
                                  scale: scaleAnimation,
                                  child: Icon(LucideLightIcons.dice_5, size: 20, color: context.colors.textSubtle),
                                ),
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                  _Section(
                    title: '포스트 상호작용',
                    children: [
                      _Option(
                        icon: LucideLightIcons.smile,
                        label: '이모지 반응',
                        trailing: HookFormSelect(
                          name: 'allowReaction',
                          initialValue: posts.first.allowReaction,
                          values: posts.map((p) => p.allowReaction).toList(),
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
                          initialValue:
                              posts.map((p) => p.protectContent).toSet().length == 1 && posts.first.protectContent,
                          values: posts.map((p) => p.protectContent).toList(),
                        ),
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
                    final box = context.findRenderObject() as RenderBox?;

                    try {
                      if (posts.length == 1) {
                        await SharePlus.instance.share(
                          ShareParams(
                            title: posts.first.title,
                            uri: Uri.parse(entities.first.url),
                            sharePositionOrigin: box!.localToGlobal(Offset.zero) & box.size,
                          ),
                        );
                      } else {
                        final urls = entities.map((e) => e.url).join('\n');
                        await SharePlus.instance.share(
                          ShareParams(
                            title: '${posts.length}개의 포스트',
                            text: urls,
                            sharePositionOrigin: box!.localToGlobal(Offset.zero) & box.size,
                          ),
                        );
                      }

                      unawaited(mixpanel.track('copy_post_share_url', properties: {'count': posts.length}));
                    } catch (_) {
                      // pass
                    }
                  },
                  child: Container(
                    decoration: BoxDecoration(
                      color: context.colors.surfaceInverse,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const Pad(vertical: 16),
                    child: Text(
                      '공유하기',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: context.colors.textInverse),
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
  }
}

class ShareFoldersContent extends HookWidget {
  const ShareFoldersContent({required this.folders, required this.entities, required this.client, super.key});

  final List<GShareEntities_QueryData_entities_node__asFolder> folders;
  final List<GShareEntities_QueryData_entities> entities;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();

    return HookForm(
      submitMode: HookFormSubmitMode.onChange,
      onSubmit: (form) async {
        final dirtyData = form.getDirtyFieldsData();
        if (dirtyData.isEmpty) {
          return;
        }

        final builder = GShareFolder_UpdateFoldersOption_MutationReqBuilder();
        builder.vars.input.folderIds.addAll(folders.map((f) => f.id));

        if (dirtyData.containsKey('visibility')) {
          builder.vars.input.visibility = Value.present(form.data['visibility'] as GEntityVisibility);
        }

        await client.request(builder.build());

        final trackProperties = <String, dynamic>{'count': folders.length};
        for (final entry in dirtyData.entries) {
          if (entry.value is EnumClass) {
            trackProperties[entry.key] = (entry.value as EnumClass).name;
          } else {
            trackProperties[entry.key] = entry.value;
          }
        }

        unawaited(mixpanel.track('update_folder_option', properties: trackProperties));
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
                      initialValue: entities.first.visibility,
                      values: entities.map((e) => e.visibility).toList(),
                      items: const [
                        HookFormSelectItem(icon: LucideLightIcons.lock, label: '비공개', value: GEntityVisibility.PRIVATE),
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
            Tappable(
              onTap: () async {
                final visibility = form.data['visibility'] as GEntityVisibility;

                await client.request(
                  GShareFolder_UpdateFoldersOption_MutationReq(
                    (b) => b
                      ..vars.input.folderIds.addAll(folders.map((f) => f.id))
                      ..vars.input.visibility = Value.present(visibility)
                      ..vars.input.recursive = const Value.present(true),
                  ),
                );

                unawaited(
                  mixpanel.track(
                    'update_folder_option',
                    properties: {'visibility': visibility.name, 'recursive': true, 'count': folders.length},
                  ),
                );

                if (context.mounted) {
                  context.toast(ToastType.success, '하위 요소에도 동일한 설정이 적용되었어요');
                  await context.router.maybePop();
                }
              },
              child: Container(
                decoration: BoxDecoration(
                  border: Border.all(color: context.colors.borderStrong),
                  borderRadius: BorderRadius.circular(8),
                ),
                padding: const Pad(vertical: 16),
                child: const Text(
                  '하위 요소에 동일한 설정 적용하기',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
                  textAlign: TextAlign.center,
                ),
              ),
            ),
            const Gap(4),
            Builder(
              builder: (context) => Tappable(
                onTap: () async {
                  final box = context.findRenderObject() as RenderBox?;

                  try {
                    if (folders.length == 1) {
                      await SharePlus.instance.share(
                        ShareParams(
                          title: folders.first.name,
                          uri: Uri.parse(entities.first.url),
                          sharePositionOrigin: box!.localToGlobal(Offset.zero) & box.size,
                        ),
                      );
                    } else {
                      final urls = entities.map((e) => e.url).join('\n');
                      await SharePlus.instance.share(
                        ShareParams(
                          title: '${folders.length}개의 폴더',
                          text: urls,
                          sharePositionOrigin: box!.localToGlobal(Offset.zero) & box.size,
                        ),
                      );
                    }

                    unawaited(mixpanel.track('copy_folder_share_url', properties: {'count': folders.length}));
                  } catch (_) {
                    // pass
                  }
                },
                child: Container(
                  decoration: BoxDecoration(
                    color: context.colors.surfaceInverse,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const Pad(vertical: 16),
                  child: Text(
                    '공유하기',
                    style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: context.colors.textInverse),
                    textAlign: TextAlign.center,
                  ),
                ),
              ),
            ),
          ],
        );
      },
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
