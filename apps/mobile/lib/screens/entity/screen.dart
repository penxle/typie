import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_menu.dart';
import 'package:typie/extensions/iterable.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/entity/__generated__/screen.data.gql.dart';
import 'package:typie/screens/entity/__generated__/screen.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

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
    return Screen(
      backgroundColor: AppColors.gray_50,
      child: entityId == null ? const _WithSiteId() : _WithEntityId(entityId!),
    );
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

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final animationController = useAnimationController(duration: const Duration(milliseconds: 150));
    final scrollController = useScrollController();
    final primaryScrollController = PrimaryScrollController.of(context);

    final isReordering = useState(false);

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
    });

    useAsyncEffect(() async {
      scrollController.jumpTo(scrollController.position.maxScrollExtent);
      return null;
    });

    const chevron = Icon(LucideIcons.chevron_right, size: 24, color: AppColors.gray_500);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        AnimatedBuilder(
          animation: animationController,
          builder: (context, child) {
            return Box(
              decoration: BoxDecoration(
                border: Border(
                  bottom: BorderSide(color: AppColors.gray_950.withValues(alpha: animationController.value)),
                ),
              ),
              padding: const Pad(vertical: 12),
              child: child,
            );
          },
          child: Row(
            children: [
              Expanded(
                child: SingleChildScrollView(
                  controller: scrollController,
                  scrollDirection: Axis.horizontal,
                  padding: const Pad(horizontal: 20),
                  child: Row(
                    children: [
                      Tappable(
                        onTap: () {
                          context.router.popUntil((route) {
                            if (route.data?.name == EntityRoute.name && route.data!.args == null) {
                              return true;
                            }

                            return false;
                          });
                        },
                        child: Text(
                          '내 포스트',
                          style: TextStyle(
                            fontSize: 24,
                            fontWeight: FontWeight.w700,
                            color: entity == null ? null : AppColors.gray_500,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (entity != null) chevron,
                      ...?entity?.ancestors
                          .map(
                            (e) => Tappable(
                              onTap: () {
                                context.router.popUntil((route) {
                                  if (route.data?.args case EntityRouteArgs(:final entityId)) {
                                    return entityId == e.id;
                                  }

                                  return false;
                                });
                              },
                              child: Text(
                                e.node.when(folder: (folder) => folder.name, orElse: () => throw UnimplementedError()),
                                style: const TextStyle(
                                  fontSize: 24,
                                  fontWeight: FontWeight.w700,
                                  color: AppColors.gray_500,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                          )
                          .intersperseWith(chevron),
                      if (entity?.ancestors.isNotEmpty ?? false) chevron,
                      if (entity != null)
                        Text(
                          entity!.node.when(folder: (folder) => folder.name, orElse: () => throw UnimplementedError()),
                          style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w700),
                          overflow: TextOverflow.ellipsis,
                        ),
                    ],
                  ),
                ),
              ),
              const Box.gap(12),
              if (isReordering.value)
                Tappable(
                  onTap: () {
                    isReordering.value = false;
                  },
                  child: const Box(
                    padding: Pad(horizontal: 12, vertical: 6),
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.all(Radius.circular(999)),
                      color: AppColors.gray_700,
                    ),
                    child: Text(
                      '완료',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.white),
                    ),
                  ),
                )
              else ...[
                Tappable(
                  padding: const Pad(all: 4),
                  onTap: () async {
                    await context.showBottomMenu(
                      items: [
                        BottomMenuItem(
                          icon: LucideIcons.chevrons_up_down,
                          label: '순서 변경하기',
                          onTap: () {
                            isReordering.value = true;
                          },
                        ),
                      ],
                    );
                  },
                  child: const Icon(LucideIcons.ellipsis, size: 24),
                ),
                const Box.gap(20),
                Tappable(padding: const Pad(all: 4), onTap: () {}, child: const Icon(LucideIcons.square_pen, size: 24)),
              ],
              const Box.gap(20),
            ],
          ),
        ),
        Expanded(
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
                    return Box(
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
                          child: Box(
                            decoration: BoxDecoration(
                              border: Border.all(color: AppColors.gray_950),
                              borderRadius: const BorderRadius.all(Radius.circular(8)),
                              color: AppColors.white,
                            ),
                            child: Row(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                if (isReordering.value)
                                  ReorderableDragStartListener(
                                    index: index,
                                    child: const Listener(
                                      behavior: HitTestBehavior.opaque,
                                      child: Box(
                                        padding: Pad(left: 16, right: 12, vertical: 12),
                                        child: Icon(LucideIcons.grip_vertical, size: 24, color: AppColors.gray_500),
                                      ),
                                    ),
                                  )
                                else
                                  const Box.gap(16),
                                Expanded(
                                  child: Box(
                                    padding: const Pad(vertical: 12),
                                    child: entities[index].node.when(
                                      folder: (_) => _Folder(entities[index]),
                                      post: (_) => _Post(entities[index]),
                                      orElse: () => throw UnimplementedError(),
                                    ),
                                  ),
                                ),
                                const Box.gap(16),
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
                ),
        ),
      ],
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
        const Icon(TypieIcons.folder_filled, size: 16),
        Expanded(
          child: Text(
            folder.name,
            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
            overflow: TextOverflow.ellipsis,
            maxLines: 1,
          ),
        ),
        const Icon(LucideIcons.chevron_right, size: 16),
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
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
            Text('${post.characterCount.humanize}자', style: const TextStyle(fontSize: 14)),
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
