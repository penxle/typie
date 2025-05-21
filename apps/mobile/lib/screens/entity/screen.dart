import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/extensions/iterable.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide.dart';
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
class EntityScreen extends HookWidget {
  const EntityScreen({super.key, @PathParam() this.entityId});

  final String? entityId;

  @override
  Widget build(BuildContext context) {
    return Screen(child: entityId == null ? const _WithSiteId() : _WithEntityId(entityId!));
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

class _WithEntityId extends HookWidget {
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
    final scrollController = useScrollController();
    final animationController = useAnimationController(duration: const Duration(milliseconds: 150));

    useEffect(() {
      void listener() {
        if (scrollController.position.pixels > 0) {
          if (animationController.status != AnimationStatus.forward) {
            animationController.forward();
          }
        } else {
          if (animationController.status != AnimationStatus.reverse) {
            animationController.reverse();
          }
        }
      }

      scrollController.addListener(listener);
      return () => scrollController.removeListener(listener);
    }, []);

    const chevron = Icon(LucideIcons.chevron_right, size: 24, color: AppColors.gray_500);

    return Column(
      children: [
        AnimatedBuilder(
          animation: animationController,
          builder: (context, child) {
            return Box(
              decoration: BoxDecoration(
                border: Border(
                  bottom: BorderSide(color: AppColors.gray_100.withValues(alpha: animationController.value)),
                ),
              ),
              padding: const Pad(horizontal: 24, vertical: 12),
              child: child,
            );
          },
          child: Row(
            children: [
              Flexible(
                child: Tappable(
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
              ),
              if (entity != null) chevron,
              ...?entity?.ancestors
                  .map(
                    (e) => Flexible(
                      child: Tappable(
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
                          style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w700, color: AppColors.gray_500),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                  )
                  .intersperseWith(chevron),
              if (entity?.ancestors.isNotEmpty ?? false) chevron,
              if (entity != null)
                Flexible(
                  child: Text(
                    entity!.node.when(folder: (folder) => folder.name, orElse: () => throw UnimplementedError()),
                    style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w700),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
            ],
          ),
        ),
        Expanded(
          child: entities.isEmpty
              ? const Center(
                  child: Text('폴더가 비어있어요', style: TextStyle(fontSize: 16, color: AppColors.gray_500)),
                )
              : ReorderableList(
                  controller: scrollController,
                  physics: const AlwaysScrollableScrollPhysics(),
                  itemCount: entities.length,
                  itemBuilder: (context, index) {
                    return ReorderableDelayedDragStartListener(
                      key: ValueKey(entities[index].id),
                      index: index,
                      child: entities[index].node.when(
                        folder: (_) => _Folder(entities[index]),
                        post: (_) => _Post(entities[index]),
                        orElse: () => throw UnimplementedError(),
                      ),
                    );
                  },
                  proxyDecorator: (child, index, animation) {
                    final curve = CurvedAnimation(parent: animation, curve: Curves.easeInOut);
                    final alpha = Tween<double>(begin: 0, end: 1).animate(curve);

                    return AnimatedBuilder(
                      animation: animation,
                      builder: (context, child) {
                        return Box(
                          decoration: BoxDecoration(
                            color: AppColors.gray_200.withValues(alpha: alpha.value),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: child,
                        );
                      },
                      child: child,
                    );
                  },
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

class _Folder extends HookWidget {
  const _Folder(this.entity);

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asFolder get folder => entity.node as GEntityScreen_Entity_entity_node__asFolder;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      child: Box(
        padding: const Pad(horizontal: 24, vertical: 12),
        child: Row(
          spacing: 8,
          children: [
            const Icon(LucideIcons.folder, size: 16, color: AppColors.gray_500),
            Expanded(
              child: Text(
                folder.name,
                style: const TextStyle(fontSize: 16, color: AppColors.gray_700),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
            Tappable(
              child: const Icon(LucideIcons.ellipsis, size: 16, color: AppColors.gray_500),
              onTap: () async {},
            ),
          ],
        ),
      ),
      onTap: () async {
        await context.router.push(EntityRoute(entityId: entity.id));
      },
    );
  }
}

class _Post extends HookWidget {
  const _Post(this.entity);

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asPost get post => entity.node as GEntityScreen_Entity_entity_node__asPost;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      child: Box(
        padding: const Pad(horizontal: 24, vertical: 12),
        child: Row(
          spacing: 8,
          children: [
            const Icon(LucideIcons.file, size: 16, color: AppColors.gray_500),
            Expanded(
              child: Text(
                post.title,
                style: const TextStyle(fontSize: 16, color: AppColors.gray_700),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
            Tappable(
              child: const Icon(LucideIcons.ellipsis, size: 16, color: AppColors.gray_500),
              onTap: () async {},
            ),
          ],
        ),
      ),
      onTap: () async {
        await context.router.push(EditorRoute(slug: entity.slug));
      },
    );
  }
}
