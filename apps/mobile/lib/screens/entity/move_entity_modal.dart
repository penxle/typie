import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/entity/__generated__/entity_fragment.data.gql.dart';
import 'package:typie/screens/entity/__generated__/move_entities_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/move_entity_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_entity_id_query.data.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_entity_id_query.req.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_site_id_query.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/widgets/tappable.dart';

const maxDepth = 3;

class MoveEntityModal extends HookWidget {
  const MoveEntityModal.single({
    super.key,
    required GEntityScreen_Entity_entity entity,
    this.onMoved,
    required this.via,
  }) : entities = null,
       singleEntity = entity;

  const MoveEntityModal.multiple({
    super.key,
    required List<GEntityScreen_Entity_entity> this.entities,
    required this.onMoved,
    required this.via,
  }) : singleEntity = null;

  final GEntityScreen_Entity_entity? singleEntity;
  final List<GEntityScreen_Entity_entity>? entities;
  final VoidCallback? onMoved;
  final String via;

  bool get isMultiple => entities != null;

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();
    final scrollController = useScrollController();

    final loading = useState(false);
    final folderEntities = useState<List<GEntityScreen_Entity_entity>?>(null);
    final currentEntity = useState<GEntityScreen_WithEntityId_QueryData_entity?>(null);

    final selectedItems = isMultiple ? entities!.map((e) => e.id).toSet() : {singleEntity!.id};

    int getMaxSelectedFolderDepth() {
      if (!isMultiple) {
        return singleEntity!.node.maybeWhen(
          folder: (folder) => folder.maxDescendantFoldersDepth - singleEntity!.depth,
          orElse: () => 0,
        );
      }

      var maxSelectedDepth = 0;
      for (final entity in entities!) {
        entity.node.maybeWhen(
          folder: (folder) {
            final internalDepth = folder.maxDescendantFoldersDepth - entity.depth;
            if (internalDepth > maxSelectedDepth) {
              maxSelectedDepth = internalDepth;
            }
          },
          orElse: () {},
        );
      }
      return maxSelectedDepth;
    }

    bool canMoveToCurrentLocation() {
      final targetDepth = (currentEntity.value?.depth ?? -1) + 1;
      final maxInternalDepth = getMaxSelectedFolderDepth();
      return targetDepth + maxInternalDepth <= maxDepth - 1;
    }

    Future<void> fetchData(String? id) async {
      loading.value = true;

      if (id != null) {
        final res = await client.request(GEntityScreen_WithEntityId_QueryReq((b) => b..vars.entityId = id));
        currentEntity.value = res.entity;
        folderEntities.value = res.entity.children.toList();
      } else {
        final res = await client.request(GEntityScreen_WithSiteId_QueryReq((b) => b..vars.siteId = pref.siteId));
        currentEntity.value = null;
        folderEntities.value = res.site.entities.toList();
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
                  const SizedBox(width: 4),
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
                      currentEntity.value!.node.maybeWhen(
                        folder: (folder) => folder.name,
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
            child: (loading.value && folderEntities.value == null)
                ? const Center(child: CircularProgressIndicator())
                : folderEntities.value!.where((element) => element.node.G__typename == 'Folder').isEmpty
                ? Center(
                    child: Text('하위 폴더가 없어요', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
                  )
                : ListView.builder(
                    itemCount: folderEntities.value!.length,
                    itemBuilder: (context, index) {
                      if (folderEntities.value![index].node.G__typename != 'Folder') {
                        return const SizedBox.shrink();
                      }

                      final isDisabled = selectedItems.contains(folderEntities.value![index].id);

                      return ListTile(
                        contentPadding: Pad.zero,
                        onTap: isDisabled
                            ? null
                            : () async {
                                if (currentEntity.value?.id != folderEntities.value![index].id) {
                                  await fetchData(folderEntities.value![index].id);
                                }
                              },
                        title: Container(
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: isDisabled ? context.colors.borderSubtle : context.colors.borderStrong,
                            ),
                            borderRadius: const BorderRadius.all(Radius.circular(8)),
                            color: isDisabled ? context.colors.surfaceMuted : context.colors.surfaceDefault,
                          ),
                          padding: const Pad(vertical: 12, horizontal: 16),
                          child: Row(
                            spacing: 8,
                            children: [
                              Icon(
                                TypieIcons.folder_filled,
                                size: 18,
                                color: isDisabled ? context.colors.textFaint : null,
                              ),
                              Expanded(
                                child: Text(
                                  folderEntities.value![index].node.maybeWhen(
                                    folder: (folder) => folder.name,
                                    orElse: () => '',
                                  ),
                                  style: TextStyle(
                                    fontSize: 16,
                                    fontWeight: FontWeight.w500,
                                    color: isDisabled ? context.colors.textFaint : null,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                  maxLines: 1,
                                ),
                              ),
                              Icon(
                                LucideLightIcons.chevron_right,
                                size: 16,
                                color: isDisabled ? context.colors.textFaint : null,
                              ),
                            ],
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
                        color: canMoveToCurrentLocation() ? context.colors.surfaceInverse : context.colors.surfaceMuted,
                        borderRadius: const BorderRadius.all(Radius.circular(999)),
                      ),
                      padding: const Pad(vertical: 12),
                      child: Tappable(
                        onTap: () async {
                          if (!canMoveToCurrentLocation()) {
                            context.toast(ToastType.error, '폴더의 최대 깊이를 초과했어요', bottom: 64);
                            return;
                          }

                          try {
                            if (isMultiple) {
                              await client.request(
                                GEntityScreen_MoveEntities_MutationReq(
                                  (b) => b
                                    ..vars.input.entityIds.addAll(selectedItems)
                                    ..vars.input.parentEntityId = currentEntity.value?.id
                                    ..vars.input.lowerOrder = folderEntities.value!.isNotEmpty
                                        ? folderEntities.value![folderEntities.value!.length - 1].order
                                        : null,
                                ),
                              );

                              unawaited(
                                mixpanel.track(
                                  'move_entities',
                                  properties: {'totalCount': selectedItems.length, 'via': via},
                                ),
                              );

                              if (context.mounted) {
                                context.toast(ToastType.success, '${selectedItems.length}개 항목이 이동되었어요');
                              }
                            } else {
                              await client.request(
                                GEntityScreen_MoveEntity_MutationReq(
                                  (b) => b
                                    ..vars.input.entityId = singleEntity!.id
                                    ..vars.input.parentEntityId = currentEntity.value?.id
                                    ..vars.input.lowerOrder = folderEntities.value!.isNotEmpty
                                        ? folderEntities.value![folderEntities.value!.length - 1].order
                                        : null,
                                ),
                              );

                              unawaited(mixpanel.track('move_entity', properties: {'via': via}));
                            }

                            if (context.mounted) {
                              await context.router.maybePop();
                              onMoved?.call();
                            }
                          } catch (_) {
                            if (context.mounted) {
                              context.toast(ToastType.error, '이동 중 오류가 발생했습니다', bottom: 64);
                            }
                          }
                        },
                        child: Text(
                          '옮기기',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w500,
                            color: canMoveToCurrentLocation() ? context.colors.textInverse : context.colors.textFaint,
                          ),
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
