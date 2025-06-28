import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:luthor/luthor.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/modals/share.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/entity/__generated__/create_folder_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/create_post_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/delete_folder_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/delete_post_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/duplicate_post_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/entity_fragment.data.gql.dart';
import 'package:typie/screens/entity/__generated__/move_entity_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/rename_folder_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_entity_id_query.data.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_entity_id_query.req.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_site_id_query.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';
import 'package:url_launcher/url_launcher.dart';

const maxDepth = 3;

@RoutePage()
class EntityRouter extends AutoRouter {
  const EntityRouter({super.key});
}

@RoutePage()
class EntityScreen extends StatelessWidget {
  const EntityScreen({super.key, @PathParam() this.entityId});

  final String? entityId;

  @override
  Widget build(BuildContext context) {
    return entityId == null ? const _WithSiteId() : _WithEntityId(entityId!);
  }
}

class _WithSiteId extends HookWidget {
  const _WithSiteId();

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GEntityScreen_WithSiteId_QueryReq((b) => b..vars.siteId = pref.siteId),
      builder: (context, client, data) {
        return _EntityList(null, data.site.entities.toList());
      },
    );
  }
}

class _WithEntityId extends StatelessWidget {
  const _WithEntityId(this.entityId);

  final String entityId;

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GEntityScreen_WithEntityId_QueryReq((b) => b..vars.entityId = entityId),
      builder: (context, client, data) {
        return _EntityList(data.entity, data.entity.children.toList());
      },
    );
  }
}

class _EntityList extends HookWidget {
  const _EntityList(this.entity, this.entities);

  final GEntityScreen_WithEntityId_QueryData_entity? entity;
  final List<GEntityScreen_Entity_entity> entities;

  GEntityScreen_WithEntityId_QueryData_entity_node__asFolder? get folder =>
      entity?.node as GEntityScreen_WithEntityId_QueryData_entity_node__asFolder?;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final pref = useService<Pref>();
    final mixpanel = useService<Mixpanel>();

    final animationController = useAnimationController(duration: const Duration(milliseconds: 150));
    final textEditingController = useTextEditingController();
    final primaryScrollController = PrimaryScrollController.of(context);

    final isReordering = useState(false);
    final isRenaming = useState(false);

    useEffect(() {
      void listener() {
        if (primaryScrollController.position.pixels > 0) {
          if (animationController.status != AnimationStatus.forward) {
            animationController.forward();
          }
        } else {
          if (animationController.status != AnimationStatus.reverse) {
            animationController.reverse();
          }
        }
      }

      primaryScrollController.addListener(listener);
      return () => primaryScrollController.removeListener(listener);
    }, [primaryScrollController]);

    useAsyncEffect(() async {
      if (isRenaming.value) {
        textEditingController.selection = TextSelection(baseOffset: 0, extentOffset: textEditingController.text.length);
      }

      return null;
    }, [isRenaming.value]);

    return HookForm(
      schema: l.schema({'name': l.string().min(1).required()}),
      onSubmit: (form) async {
        await client.request(
          GEntityScreen_RenameFolder_MutationReq(
            (b) => b
              ..vars.input.folderId = folder!.id
              ..vars.input.name = form.data['name'] as String,
          ),
        );

        unawaited(mixpanel.track('rename_folder'));
        isRenaming.value = false;
      },
      builder: (context, form) {
        return Screen(
          heading: Heading(
            titleWidget: Row(
              spacing: 8,
              children: [
                Icon(entity == null ? LucideLightIcons.folder_open : LucideLightIcons.folder, size: 20),
                Expanded(
                  child: isRenaming.value
                      ? HookFormTextField.collapsed(
                          name: 'name',
                          controller: textEditingController,
                          autofocus: true,
                          initialValue: folder!.name,
                          placeholder: '폴더 이름',
                          style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                        )
                      : GestureDetector(
                          onDoubleTap: () {
                            if (entity != null) {
                              isRenaming.value = true;
                            }
                          },
                          child: Text(
                            entity == null
                                ? '내 포스트'
                                : textEditingController.text.isEmpty
                                ? folder!.name
                                : textEditingController.text,
                            style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                ),
              ],
            ),
            leadingWidget: isRenaming.value
                ? HeadingLeading(
                    icon: LucideLightIcons.x,
                    onTap: () {
                      isRenaming.value = false;
                      textEditingController.text = '';
                    },
                  )
                : null,
            actions: [
              if (!isRenaming.value && !isReordering.value)
                HeadingAction(
                  icon: LucideLightIcons.ellipsis,
                  onTap: () async {
                    await context.showBottomSheet(
                      child: BottomMenu(
                        header: _BottomMenuHeader(entity: entity),
                        items: [
                          if (entity != null) ...[
                            BottomMenuItem(
                              icon: LucideLightIcons.folder_symlink,
                              label: '다른 폴더로 옮기기',
                              onTap: () async {
                                unawaited(mixpanel.track('move_entity_try', properties: {'via': 'entity_menu'}));

                                await context.showBottomSheet(intercept: true, child: _MoveEntityModal(entity!));
                              },
                            ),
                            BottomMenuItem(
                              icon: LucideLightIcons.external_link,
                              label: '사이트에서 열기',
                              onTap: () async {
                                unawaited(mixpanel.track('open_folder_in_browser', properties: {'via': 'entity_menu'}));

                                final url = Uri.parse(entity!.url);
                                await launchUrl(url, mode: LaunchMode.externalApplication);
                              },
                            ),
                            BottomMenuItem(
                              icon: LucideLightIcons.blend,
                              label: '공유하기',
                              onTap: () async {
                                unawaited(
                                  mixpanel.track('open_folder_share_modal', properties: {'via': 'entity_menu'}),
                                );

                                await context.showBottomSheet(
                                  intercept: true,
                                  child: ShareFolderBottomSheet(entityId: entity!.id),
                                );
                              },
                            ),
                          ],
                          BottomMenuItem(
                            icon: LucideLightIcons.square_pen,
                            label: '하위 포스트 만들기',
                            onTap: () async {
                              final resp = await client.request(
                                GEntityScreen_CreatePost_MutationReq(
                                  (b) => b
                                    ..vars.input.siteId = pref.siteId
                                    ..vars.input.parentEntityId = entity?.id,
                                ),
                              );

                              unawaited(mixpanel.track('create_post', properties: {'via': 'entity_menu'}));

                              if (context.mounted) {
                                await context.router.push(EditorRoute(slug: resp.createPost.entity.slug));
                              }
                            },
                          ),
                          if ((entity?.depth ?? 0) < maxDepth - 1)
                            BottomMenuItem(
                              icon: LucideLightIcons.folder_plus,
                              label: '하위 폴더 만들기',
                              onTap: () async {
                                final resp = await client.request(
                                  GEntityScreen_CreateFolder_MutationReq(
                                    (b) => b
                                      ..vars.input.siteId = pref.siteId
                                      ..vars.input.parentEntityId = entity?.id
                                      ..vars.input.name = '새 폴더',
                                  ),
                                );

                                unawaited(mixpanel.track('create_folder'));

                                if (context.mounted) {
                                  await context.router.push(EntityRoute(entityId: resp.createFolder.entity.id));
                                }
                              },
                            ),
                          if (entities.length > 1)
                            BottomMenuItem(
                              icon: LucideLightIcons.chevrons_up_down,
                              label: '순서 변경하기',
                              onTap: () {
                                isReordering.value = true;
                              },
                            ),
                          if (entity != null) ...[
                            BottomMenuItem(
                              icon: LucideLightIcons.pen_line,
                              label: '이름 바꾸기',
                              onTap: () {
                                isRenaming.value = true;
                              },
                            ),
                            BottomMenuItem(
                              icon: LucideLightIcons.trash,
                              label: '삭제하기',
                              onTap: () async {
                                await context.showModal(
                                  child: ConfirmModal(
                                    title: '폴더 삭제',
                                    message: '"${folder!.name}" 폴더를 삭제하시겠어요?',
                                    confirmText: '삭제하기',
                                    confirmTextColor: context.colors.textBright,
                                    confirmBackgroundColor: context.colors.accentDanger,
                                    onConfirm: () async {
                                      await client.request(
                                        GEntityScreen_DeleteFolder_MutationReq(
                                          (b) => b..vars.input.folderId = folder!.id,
                                        ),
                                      );

                                      unawaited(mixpanel.track('delete_folder'));

                                      if (context.mounted) {
                                        await context.router.maybePop();
                                      }
                                    },
                                  ),
                                );
                              },
                            ),
                          ],
                        ],
                      ),
                    );
                  },
                )
              else
                HeadingAction(
                  icon: LucideLightIcons.check,
                  onTap: () async {
                    if (isRenaming.value) {
                      await form.submit();
                    } else if (isReordering.value) {
                      isReordering.value = false;
                    }
                  },
                ),
            ],
          ),
          child: entities.isEmpty
              ? Center(
                  child: Text(
                    '폴더가 비어있어요',
                    style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                  ),
                )
              : ReorderableList(
                  controller: primaryScrollController,
                  physics: const AlwaysScrollableScrollPhysics(),
                  padding: const Pad(horizontal: 20, vertical: 14),
                  itemCount: entities.length,
                  itemBuilder: (context, index) {
                    return Padding(
                      key: Key(entities[index].id),
                      padding: const Pad(vertical: 6),
                      child: GestureDetector(
                        onTap: () async {
                          if (isReordering.value) {
                            return;
                          }

                          await entities[index].node.when(
                            folder: (folder) => context.router.push(EntityRoute(entityId: entities[index].id)),
                            post: (post) => context.router.push(EditorRoute(slug: entities[index].slug)),
                            orElse: () => throw UnimplementedError(),
                          );
                        },
                        onLongPress: () async {
                          if (isReordering.value) {
                            return;
                          }

                          await entities[index].node.when(
                            folder: (folder) => context.showBottomSheet(
                              child: BottomMenu(
                                header: _BottomMenuHeader(entity: entities[index]),
                                items: [
                                  BottomMenuItem(
                                    icon: LucideLightIcons.folder_symlink,
                                    label: '다른 폴더로 옮기기',
                                    onTap: () async {
                                      unawaited(
                                        mixpanel.track('move_entity_try', properties: {'via': 'entity_folder_menu'}),
                                      );

                                      await context.showBottomSheet(
                                        intercept: true,
                                        child: _MoveEntityModal(entities[index]),
                                      );
                                    },
                                  ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.external_link,
                                    label: '사이트에서 열기',
                                    onTap: () async {
                                      unawaited(
                                        mixpanel.track(
                                          'open_folder_in_browser',
                                          properties: {'via': 'entity_folder_menu'},
                                        ),
                                      );

                                      final url = Uri.parse(entities[index].url);
                                      await launchUrl(url, mode: LaunchMode.externalApplication);
                                    },
                                  ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.blend,
                                    label: '공유하기',
                                    onTap: () async {
                                      unawaited(
                                        mixpanel.track(
                                          'open_folder_share_modal',
                                          properties: {'via': 'entity_folder_menu'},
                                        ),
                                      );

                                      await context.showBottomSheet(
                                        intercept: true,
                                        child: ShareFolderBottomSheet(entityId: entities[index].id),
                                      );
                                    },
                                  ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.square_pen,
                                    label: '하위 포스트 만들기',
                                    onTap: () async {
                                      final resp = await client.request(
                                        GEntityScreen_CreatePost_MutationReq(
                                          (b) => b
                                            ..vars.input.siteId = pref.siteId
                                            ..vars.input.parentEntityId = entities[index].id,
                                        ),
                                      );

                                      unawaited(
                                        mixpanel.track('create_post', properties: {'via': 'entity_folder_menu'}),
                                      );

                                      if (context.mounted) {
                                        await context.router.push(EditorRoute(slug: resp.createPost.entity.slug));
                                      }
                                    },
                                  ),
                                  if (entities[index].depth < maxDepth - 1)
                                    BottomMenuItem(
                                      icon: LucideLightIcons.folder_plus,
                                      label: '하위 폴더 만들기',
                                      onTap: () async {
                                        final resp = await client.request(
                                          GEntityScreen_CreateFolder_MutationReq(
                                            (b) => b
                                              ..vars.input.siteId = pref.siteId
                                              ..vars.input.parentEntityId = entities[index].id
                                              ..vars.input.name = '새 폴더',
                                          ),
                                        );

                                        unawaited(
                                          mixpanel.track('create_folder', properties: {'via': 'entity_folder_menu'}),
                                        );

                                        if (context.mounted) {
                                          await context.router.push(EntityRoute(entityId: resp.createFolder.entity.id));
                                        }
                                      },
                                    ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.trash,
                                    label: '삭제하기',
                                    onTap: () async {
                                      await context.showModal(
                                        child: ConfirmModal(
                                          title: '폴더 삭제',
                                          message: '"${folder.name}" 폴더를 삭제하시겠어요?',
                                          confirmText: '삭제하기',
                                          confirmTextColor: context.colors.textBright,
                                          confirmBackgroundColor: context.colors.accentDanger,
                                          onConfirm: () async {
                                            await client.request(
                                              GEntityScreen_DeleteFolder_MutationReq(
                                                (b) => b..vars.input.folderId = folder.id,
                                              ),
                                            );

                                            unawaited(
                                              mixpanel.track(
                                                'delete_folder',
                                                properties: {'via': 'entity_folder_menu'},
                                              ),
                                            );
                                          },
                                        ),
                                      );
                                    },
                                  ),
                                ],
                              ),
                            ),
                            post: (post) => context.showBottomSheet(
                              child: BottomMenu(
                                header: _BottomMenuHeader(entity: entities[index]),
                                items: [
                                  BottomMenuItem(
                                    icon: LucideLightIcons.file_symlink,
                                    label: '다른 폴더로 옮기기',
                                    onTap: () async {
                                      unawaited(
                                        mixpanel.track('move_entity_try', properties: {'via': 'entity_post_menu'}),
                                      );

                                      await context.showBottomSheet(
                                        intercept: true,
                                        child: _MoveEntityModal(entities[index]),
                                      );
                                    },
                                  ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.external_link,
                                    label: '사이트에서 열기',
                                    onTap: () async {
                                      unawaited(
                                        mixpanel.track('open_post_in_browser', properties: {'via': 'entity_post_menu'}),
                                      );

                                      final url = Uri.parse(entities[index].url);
                                      await launchUrl(url, mode: LaunchMode.externalApplication);
                                    },
                                  ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.blend,
                                    label: '공유하기',
                                    onTap: () async {
                                      unawaited(
                                        mixpanel.track(
                                          'open_post_share_modal',
                                          properties: {'via': 'entity_post_menu'},
                                        ),
                                      );

                                      await context.showBottomSheet(
                                        intercept: true,
                                        child: SharePostBottomSheet(slug: entities[index].slug),
                                      );
                                    },
                                  ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.copy,
                                    label: '복제하기',
                                    onTap: () async {
                                      await client.request(
                                        GEntityScreen_DuplicatePost_MutationReq((b) => b..vars.input.postId = post.id),
                                      );

                                      unawaited(
                                        mixpanel.track('duplicate_post', properties: {'via': 'entity_post_menu'}),
                                      );
                                    },
                                  ),
                                  BottomMenuItem(
                                    icon: LucideLightIcons.trash,
                                    label: '삭제하기',
                                    onTap: () async {
                                      await context.showModal(
                                        intercept: true,
                                        child: ConfirmModal(
                                          title: '포스트 삭제',
                                          message: '"${post.title}" 포스트를 삭제하시겠어요?',
                                          confirmText: '삭제하기',
                                          confirmTextColor: context.colors.textBright,
                                          confirmBackgroundColor: context.colors.accentDanger,
                                          onConfirm: () async {
                                            await client.request(
                                              GEntityScreen_DeletePost_MutationReq(
                                                (b) => b..vars.input.postId = post.id,
                                              ),
                                            );

                                            unawaited(
                                              mixpanel.track('delete_post', properties: {'via': 'entity_post_menu'}),
                                            );
                                          },
                                        ),
                                      );
                                    },
                                  ),
                                ],
                              ),
                            ),
                            orElse: () => throw UnimplementedError(),
                          );
                        },
                        child: IntrinsicHeight(
                          child: DecoratedBox(
                            decoration: BoxDecoration(
                              border: Border.all(color: context.colors.borderStrong),
                              borderRadius: const BorderRadius.all(Radius.circular(8)),
                              color: context.colors.surfaceDefault,
                            ),
                            child: Row(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                if (isReordering.value) ...[
                                  ReorderableDragStartListener(
                                    index: index,
                                    child: const Listener(
                                      behavior: HitTestBehavior.opaque,
                                      child: Padding(
                                        padding: Pad(all: 12),
                                        child: Icon(LucideLightIcons.grip_vertical, size: 20),
                                      ),
                                    ),
                                  ),
                                  AppVerticalDivider(color: context.colors.borderStrong),
                                ],
                                const Gap(16),
                                Expanded(
                                  child: Padding(
                                    padding: const Pad(vertical: 12),
                                    child: entities[index].node.when(
                                      folder: (_) => _Folder(entities[index]),
                                      post: (_) => _Post(entities[index]),
                                      orElse: () => throw UnimplementedError(),
                                    ),
                                  ),
                                ),
                                const Gap(16),
                              ],
                            ),
                          ),
                        ),
                      ),
                    );
                  },
                  proxyDecorator: (child, index, animation) => child,
                  onReorder: (oldIndex, newIndex) async {
                    final dragging = entities[oldIndex];
                    String? lowerOrder;
                    String? upperOrder;

                    if (newIndex >= entities.length) {
                      lowerOrder = entities[entities.length - 1].order;
                      entities
                        ..remove(dragging)
                        ..add(dragging);
                    } else if (newIndex == 0) {
                      upperOrder = entities[0].order;
                      entities
                        ..remove(dragging)
                        ..insert(newIndex, dragging);
                    } else {
                      lowerOrder = entities[newIndex - 1].order;
                      upperOrder = entities[newIndex].order;

                      if (oldIndex > newIndex) {
                        entities
                          ..removeAt(oldIndex)
                          ..insert(newIndex, dragging);
                      } else {
                        entities
                          ..remove(dragging)
                          ..insert(newIndex - 1, dragging);
                      }
                    }

                    await client.request(
                      GEntityScreen_MoveEntity_MutationReq(
                        (b) => b
                          ..vars.input.entityId = dragging.id
                          ..vars.input.parentEntityId = entity?.id
                          ..vars.input.lowerOrder = lowerOrder
                          ..vars.input.upperOrder = upperOrder
                          ..vars.input.treatEmptyParentIdAsRoot = true,
                      ),
                    );

                    unawaited(mixpanel.track('move_entity', properties: {'via': 'reorder'}));
                  },
                  onReorderStart: (index) async {
                    await HapticFeedback.lightImpact();
                  },
                  onReorderEnd: (index) async {
                    await HapticFeedback.lightImpact();
                  },
                ),
        );
      },
    );
  }
}

class _Folder extends StatelessWidget {
  const _Folder(this.entity, {this.color});

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asFolder get folder => entity.node as GEntityScreen_Entity_entity_node__asFolder;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 8,
      children: [
        Icon(TypieIcons.folder_filled, size: 18, color: color),
        Expanded(
          child: Text(
            folder.name,
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: color),
            overflow: TextOverflow.ellipsis,
            maxLines: 1,
          ),
        ),
        const Icon(LucideLightIcons.chevron_right, size: 16),
      ],
    );
  }
}

class _Post extends StatelessWidget {
  const _Post(this.entity);

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asPost get post => entity.node as GEntityScreen_Entity_entity_node__asPost;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 4,
      children: [
        Row(
          spacing: 8,
          children: [
            if (post.type == GPostType.TEMPLATE) const Icon(LucideLightIcons.shapes, size: 18),
            Expanded(
              child: Text(
                post.title,
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
            if (post.type == GPostType.NORMAL)
              Text(post.updatedAt.fromNow(), style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
          ],
        ),
        Text(
          post.excerpt.isEmpty ? '(내용 없음)' : post.excerpt,
          style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
          overflow: TextOverflow.ellipsis,
          maxLines: 1,
        ),
      ],
    );
  }
}

class _BottomMenuHeader extends StatelessWidget {
  const _BottomMenuHeader({this.entity});

  final GEntityScreen_Entity_entity? entity;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Row(
          spacing: 16,
          children: [
            Icon(
              entity?.node.when(
                    folder: (_) => LucideLightIcons.folder,
                    post: (_) => LucideLightIcons.file,
                    orElse: () => throw UnimplementedError(),
                  ) ??
                  LucideLightIcons.folder_open,
              size: 20,
            ),
            Expanded(
              child: Text(
                entity?.node.when(
                      folder: (folder) => folder.name,
                      post: (post) => post.title,
                      orElse: () => throw UnimplementedError(),
                    ) ??
                    '내 포스트',
                style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
          ],
        ),
        Padding(
          padding: const Pad(left: 36),
          child: Row(
            children: [
              Text('내 포스트', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
              ...?entity?.ancestors
                  .map(
                    (ancestor) => [
                      const Icon(LucideLightIcons.chevron_right, size: 14),
                      Text(
                        ancestor.node.when(folder: (folder) => folder.name, orElse: () => throw UnimplementedError()),
                        style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                      ),
                    ],
                  )
                  .flattened,
            ],
          ),
        ),
      ],
    );
  }
}

class _MoveEntityModal extends HookWidget {
  const _MoveEntityModal(this.entity);

  final GEntityScreen_Entity_entity entity;

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();
    final scrollController = useScrollController();

    final loading = useState(false);
    final entities = useState<List<GEntityScreen_Entity_entity>?>(null);
    final currentEntity = useState<GEntityScreen_WithEntityId_QueryData_entity?>(null);

    Future<void> fetchData(String? id) async {
      loading.value = true;

      if (id != null) {
        final res = await client.request(GEntityScreen_WithEntityId_QueryReq((b) => b..vars.entityId = id));
        currentEntity.value = res.entity;
        entities.value = res.entity.children.toList();
      } else {
        final res = await client.request(GEntityScreen_WithSiteId_QueryReq((b) => b..vars.siteId = pref.siteId));
        currentEntity.value = null;
        entities.value = res.site.entities.toList();
      }
      loading.value = false;
    }

    useEffect(() {
      unawaited(fetchData(null));

      return null;
    }, []);

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (scrollController.hasClients) {
          await scrollController.animateTo(
            scrollController.position.maxScrollExtent,
            duration: const Duration(milliseconds: 300),
            curve: Curves.easeOut,
          );
        }
      });

      return null;
    }, [currentEntity.value]);

    return AppFullBottomSheet(
      title: '다른 폴더로 옮기기',
      child: Stack(
        children: [
          Positioned(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              controller: scrollController,
              child: Row(
                spacing: 4,
                children: [
                  const Icon(LucideLightIcons.folder_open, size: 18),
                  const Gap(4),
                  Tappable(
                    onTap: () async {
                      if (currentEntity.value != null) {
                        await fetchData(null);
                      }
                    },
                    child: Text(
                      '내 포스트',
                      style: TextStyle(fontWeight: currentEntity.value == null ? FontWeight.w600 : null),
                    ),
                  ),
                  if (currentEntity.value != null) ...[
                    ...currentEntity.value!.ancestors
                        .map(
                          (ancestor) => [
                            const Icon(LucideLightIcons.chevron_right, size: 14),
                            Tappable(
                              onTap: () async {
                                await fetchData(ancestor.id);
                              },
                              child: Text(
                                ancestor.node.when(
                                  folder: (folder) => folder.name,
                                  orElse: () => throw UnimplementedError(),
                                ),
                              ),
                            ),
                          ],
                        )
                        .expand((e) => e),
                    const Icon(LucideLightIcons.chevron_right, size: 14),
                    Text(
                      currentEntity.value!.node.when(
                        folder: (folder) => folder.name,
                        post: (_) => throw UnimplementedError(),
                        orElse: () => throw UnimplementedError(),
                      ),
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                  ],
                ],
              ),
            ),
          ),
          Padding(
            padding: const Pad(top: 40, bottom: 20),
            child: (loading.value && entities.value == null)
                ? const Center(child: CircularProgressIndicator())
                : entities.value!.where((element) => element.node.G__typename == 'Folder').isEmpty
                ? Center(
                    child: Text('하위 폴더가 없어요', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
                  )
                : ListView.builder(
                    itemCount: entities.value!.length,
                    itemBuilder: (context, index) {
                      if (entities.value![index].node.G__typename != 'Folder') {
                        return const SizedBox.shrink();
                      }

                      return ListTile(
                        contentPadding: Pad.zero,
                        onTap: () async {
                          if (entity.id == entities.value![index].id) {
                            return;
                          }

                          if (currentEntity.value?.id != entities.value![index].id) {
                            await fetchData(entities.value![index].id);
                          }
                        },
                        title: Container(
                          decoration: BoxDecoration(
                            border: Border.all(color: context.colors.borderStrong),
                            borderRadius: const BorderRadius.all(Radius.circular(8)),
                            color: entity.id == entities.value![index].id
                                ? context.colors.surfaceMuted
                                : context.colors.surfaceDefault,
                          ),
                          padding: const Pad(vertical: 12, horizontal: 16),
                          child: entities.value![index].node.when(
                            folder: (folder) => _Folder(
                              entities.value![index],
                              color: entity.id == entities.value![index].id ? context.colors.textFaint : null,
                            ),
                            post: (_) => const SizedBox.shrink(),
                            orElse: () => throw UnimplementedError(),
                          ),
                        ),
                      );
                    },
                  ),
          ),
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            child: Container(
              decoration: BoxDecoration(color: context.colors.surfaceDefault),
              padding: const Pad(horizontal: 20, vertical: 4),
              child: Row(
                spacing: 8,
                children: [
                  Expanded(
                    child: Container(
                      decoration: BoxDecoration(
                        color: context.colors.surfaceMuted,
                        borderRadius: const BorderRadius.all(Radius.circular(999)),
                      ),
                      padding: const Pad(vertical: 12),
                      child: Tappable(
                        onTap: () async {
                          await context.router.maybePop();
                        },
                        child: const Text(
                          '취소',
                          textAlign: TextAlign.center,
                          style: TextStyle(fontWeight: FontWeight.w500),
                        ),
                      ),
                    ),
                  ),
                  Expanded(
                    child: Container(
                      decoration: BoxDecoration(
                        color: entity.node.when(
                          folder: (folder) =>
                              (currentEntity.value == null ? 0 : currentEntity.value!.depth + 1) +
                                      (folder.maxDescendantFoldersDepth - entity.depth) >
                                  (maxDepth - 1)
                              ? context.colors.surfaceMuted
                              : context.colors.surfaceInverse,
                          post: (_) => context.colors.surfaceInverse,
                          orElse: () => throw UnimplementedError(),
                        ),
                        borderRadius: const BorderRadius.all(Radius.circular(999)),
                      ),
                      padding: const Pad(vertical: 12),
                      child: Tappable(
                        onTap: () async {
                          entity.node.when(
                            folder: (folder) {
                              if ((currentEntity.value == null ? 0 : currentEntity.value!.depth + 1) +
                                      (folder.maxDescendantFoldersDepth - entity.depth) >
                                  (maxDepth - 1)) {
                                context.toast(ToastType.error, '폴더의 최대 깊이를 초과했어요');
                                return;
                              }
                            },
                            post: (_) {},
                            orElse: () => throw UnimplementedError(),
                          );

                          await client.request(
                            GEntityScreen_MoveEntity_MutationReq(
                              (b) => b
                                ..vars.input.entityId = entity.id
                                ..vars.input.parentEntityId = currentEntity.value?.id
                                ..vars.input.lowerOrder = entities.value!.isNotEmpty
                                    ? entities.value![entities.value!.length - 1].order
                                    : null
                                ..vars.input.treatEmptyParentIdAsRoot = true,
                            ),
                          );

                          unawaited(mixpanel.track('move_entity', properties: {'via': 'modal'}));

                          if (context.mounted) {
                            await context.router.maybePop();
                          }
                        },
                        child: Text(
                          '옮기기',
                          textAlign: TextAlign.center,
                          style: TextStyle(fontWeight: FontWeight.w500, color: context.colors.textInverse),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
