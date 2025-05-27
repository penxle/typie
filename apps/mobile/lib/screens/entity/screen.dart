import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:luthor/luthor.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/entity/__generated__/screen.data.gql.dart';
import 'package:typie/screens/entity/__generated__/screen.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';

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
                        items: [
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

                              if (context.mounted) {
                                await context.router.push(EditorRoute(slug: resp.createPost.entity.slug));
                              }
                            },
                          ),
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
                                    confirmColor: AppColors.red_500,
                                    onConfirm: () async {
                                      await client.request(
                                        GEntityScreen_DeleteFolder_MutationReq(
                                          (b) => b..vars.input.folderId = folder!.id,
                                        ),
                                      );

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
              ? const Center(
                  child: Text(
                    '폴더가 비어있어요',
                    style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.gray_500),
                  ),
                )
              : ReorderableList(
                  controller: primaryScrollController,
                  physics: const AlwaysScrollableScrollPhysics(),
                  padding: const Pad(horizontal: 20, vertical: 12),
                  itemCount: entities.length,
                  itemBuilder: (context, index) {
                    return Padding(
                      key: Key(entities[index].id),
                      padding: const Pad(vertical: 6),
                      child: Tappable(
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
                        child: IntrinsicHeight(
                          child: DecoratedBox(
                            decoration: BoxDecoration(
                              border: Border.all(color: AppColors.gray_950),
                              borderRadius: const BorderRadius.all(Radius.circular(8)),
                              color: AppColors.white,
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
                                        padding: Pad(horizontal: 12, vertical: 12),
                                        child: Icon(
                                          LucideLightIcons.grip_vertical,
                                          size: 20,
                                          color: AppColors.gray_950,
                                        ),
                                      ),
                                    ),
                                  ),
                                  const AppVerticalDivider(color: AppColors.gray_950),
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
                          ..vars.input.lowerOrder = lowerOrder
                          ..vars.input.upperOrder = upperOrder,
                      ),
                    );
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
  const _Folder(this.entity);

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asFolder get folder => entity.node as GEntityScreen_Entity_entity_node__asFolder;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 8,
      children: [
        const Icon(TypieIcons.folder_filled, size: 18),
        Expanded(
          child: Text(
            folder.name,
            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
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
            Expanded(
              child: Text(
                post.title,
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
            Text(post.updatedAt.fromNow(), style: const TextStyle(fontSize: 14, color: AppColors.gray_700)),
          ],
        ),
        Text(
          post.excerpt.isEmpty ? '(내용 없음)' : post.excerpt,
          style: const TextStyle(fontSize: 14, color: AppColors.gray_700),
          overflow: TextOverflow.ellipsis,
          maxLines: 1,
        ),
      ],
    );
  }
}
