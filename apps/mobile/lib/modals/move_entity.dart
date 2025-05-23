import 'dart:async';

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/modals/__generated__/move_entity.data.gql.dart';
import 'package:typie/modals/__generated__/move_entity.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/btn.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

class MoveEntityModal extends HookWidget {
  const MoveEntityModal(this.entityId, {this.depth = -1, super.key});

  final String entityId;
  final int depth;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();

    final entities = useState<GMoveEntityModal_QueryData?>(null);
    final rootEntities = useState<GMoveEntityModal_Root_QueryData?>(null);
    final loading = useState(false);
    final currentParentId = useState<String?>(null);

    const maxDepth = 3;

    Future<void> fetchData(String? id) async {
      loading.value = true;

      if (id != null) {
        final res = await client.request(GMoveEntityModal_QueryReq((b) => b..vars.entityId = id));
        loading.value = false;
        entities.value = res;
      } else {
        final res = await client.request(GMoveEntityModal_Root_QueryReq());
        loading.value = false;
        rootEntities.value = res;
      }
    }

    useEffect(() {
      unawaited(fetchData(null));

      return null;
    }, []);

    var folderCount = 0;

    final children = entities.value?.entity.children.toList() ?? [];
    final rootChildren = rootEntities.value?.me?.sites[0].entities.toList() ?? [];
    final targetDepth = currentParentId.value == null ? 0 : (entities.value?.entity.depth ?? 0) + 1;
    final exceedMaxDepth = (depth + targetDepth) >= maxDepth;

    folderCount = currentParentId.value != null
        ? children.where((child) => child.node.G__typename == 'Folder').length
        : rootChildren.where((child) => child.node.G__typename == 'Folder').length;

    return Screen(
      heading: Heading(
        titleWidget: Text(
          (entities.value?.entity.node.G__typename == 'Folder' && currentParentId.value != null)
              ? (entities.value!.entity.node as GMoveEntityModal_QueryData_entity_node__asFolder).name
              : '다른 폴더로 이동',
          style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
          overflow: TextOverflow.ellipsis,
          maxLines: 1,
        ),
        leading: (entities.value?.entity.parent == null && currentParentId.value == null)
            ? const Text('')
            : Tappable(
                child: const Icon(LucideIcons.chevron_left),
                onTap: () async {
                  if (entities.value?.entity.parent != null) {
                    await fetchData(entities.value!.entity.parent!.id);
                  } else if (currentParentId.value != null) {
                    currentParentId.value = null;
                    entities.value = null;
                    await fetchData(null);
                  }
                },
              ),
      ),
      child: Column(
        children: [
          Expanded(
            child: (entities.value == null && rootEntities.value == null) || loading.value
                ? const Center(child: CircularProgressIndicator())
                : Expanded(
                    child: (folderCount == 0)
                        ? const Center(
                            child: Text('폴더가 비어있어요', style: TextStyle(fontSize: 16, color: Colors.grey)),
                          )
                        : ListView.builder(
                            itemCount: (currentParentId.value != null ? children : rootChildren).length,
                            itemBuilder: (context, index) {
                              final id = currentParentId.value != null ? children[index].id : rootChildren[index].id;
                              final typename = currentParentId.value != null
                                  ? children[index].node.G__typename
                                  : rootChildren[index].node.G__typename;

                              if (typename != 'Folder') {
                                return const SizedBox.shrink();
                              }

                              final name = currentParentId.value != null
                                  ? (children[index].node as GMoveEntityModal_QueryData_entity_children_node__asFolder)
                                        .name
                                  : (rootChildren[index].node
                                            as GMoveEntityModal_Root_QueryData_me_sites_entities_node__asFolder)
                                        .name;

                              return ListTile(
                                title: Row(
                                  spacing: 8,
                                  children: [
                                    Icon(
                                      LucideIcons.folder,
                                      size: 18,
                                      color: exceedMaxDepth || id == entityId ? AppColors.gray_400 : null,
                                    ),
                                    Expanded(
                                      child: Text(
                                        name,
                                        overflow: TextOverflow.ellipsis,
                                        maxLines: 1,
                                        style: exceedMaxDepth || id == entityId
                                            ? const TextStyle(color: AppColors.gray_400)
                                            : null,
                                      ),
                                    ),
                                  ],
                                ),
                                contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                                onTap: () async {
                                  if (exceedMaxDepth || id == entityId) {
                                    return;
                                  }

                                  currentParentId.value = id;
                                  await fetchData(id);
                                },
                              );
                            },
                          ),
                  ),
          ),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.only(left: 24, right: 24, top: 12),
            decoration: const BoxDecoration(
              color: Colors.white,
              border: Border(top: BorderSide(color: AppColors.gray_100)),
            ),
            child: Row(
              spacing: 8,
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                SizedBox(
                  width: 64,
                  child: Btn(
                    '취소',
                    onTap: () {
                      context.router.pop();
                    },
                  ),
                ),
                SizedBox(
                  width: 64,
                  child: Btn(
                    '이동',
                    variant: exceedMaxDepth ? BtnVariant.disabled : BtnVariant.primary,
                    onTap: () async {
                      if (exceedMaxDepth) {
                        context.toast(ToastType.error, '최대 깊이를 초과했어요.');
                        return;
                      }

                      final lowerOrder = currentParentId.value != null
                          ? children[children.length - 1].order
                          : rootChildren[children.length - 1].order;

                      context.router.pop();
                      await client.request(
                        GMoveEntityModal_MoveEntity_MutationReq(
                          (b) => b
                            ..vars.input.entityId = entityId
                            ..vars.input.parentEntityId = currentParentId.value
                            ..vars.input.lowerOrder = lowerOrder,
                        ),
                      );
                    },
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
