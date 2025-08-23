import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/share.dart';
import 'package:typie/screens/entity/__generated__/delete_entities_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/entity_fragment.data.gql.dart';
import 'package:typie/screens/entity/move_entity_modal.dart';

class MultiEntitiesMenu extends HookWidget {
  const MultiEntitiesMenu({
    super.key,
    required this.selectedItems,
    required this.entities,
    required this.onExitSelectionMode,
    required this.via,
  });

  final Set<String> selectedItems;
  final List<GEntityScreen_Entity_entity> entities;
  final VoidCallback onExitSelectionMode;
  final String via;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();

    final selectedEntities = entities.where((e) => selectedItems.contains(e.id)).toList();

    final folderIds = selectedEntities.where((e) => e.node.G__typename == 'Folder').map((e) => e.id).toList();
    final postIds = selectedEntities.where((e) => e.node.G__typename == 'Post').map((e) => e.id).toList();
    final canvasIds = selectedEntities.where((e) => e.node.G__typename == 'Canvas').map((e) => e.id).toList();

    return BottomMenu(
      header: MultiEntitiesMenuHeader(
        selectedCount: selectedItems.length,
        folderCount: folderIds.length,
        postCount: postIds.length,
        canvasCount: canvasIds.length,
      ),
      items: [
        if (folderIds.isNotEmpty)
          BottomMenuItem(
            icon: LucideLightIcons.blend,
            label: '폴더 ${folderIds.length}개 공유 및 게시',
            onTap: () async {
              unawaited(
                mixpanel.track(
                  'open_folder_share_modal',
                  properties: {'via': 'multi_entities_menu', 'count': folderIds.length},
                ),
              );
              await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: folderIds));
            },
          ),
        if (postIds.isNotEmpty)
          BottomMenuItem(
            icon: LucideLightIcons.blend,
            label: '포스트 ${postIds.length}개 공유 및 게시',
            onTap: () async {
              unawaited(
                mixpanel.track(
                  'open_post_share_modal',
                  properties: {'via': 'multi_entities_menu', 'count': postIds.length},
                ),
              );
              await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: postIds));
            },
          ),
        BottomMenuItem(
          icon: LucideLightIcons.folder_symlink,
          label: '다른 폴더로 옮기기',
          onTap: () async {
            unawaited(
              mixpanel.track('move_entities_try', properties: {'totalCount': selectedItems.length, 'via': via}),
            );
            await context.showBottomSheet(
              intercept: true,
              child: MoveEntityModal.multiple(entities: selectedEntities, via: via, onMoved: onExitSelectionMode),
            );
          },
        ),
        BottomMenuItem(
          icon: LucideLightIcons.trash_2,
          label: '삭제하기',
          onTap: () async {
            await context.showModal(
              child: ConfirmModal(
                title: '선택한 항목 삭제',
                message: '선택한 ${selectedItems.length}개 항목을 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
                confirmText: '삭제하기',
                confirmTextColor: context.colors.textBright,
                confirmBackgroundColor: context.colors.accentDanger,
                onConfirm: () async {
                  try {
                    await client.request(
                      GEntityScreen_DeleteEntities_MutationReq((b) => b..vars.input.entityIds.addAll(selectedItems)),
                    );

                    unawaited(
                      mixpanel.track('delete_entities', properties: {'totalCount': selectedItems.length, 'via': via}),
                    );

                    if (context.mounted) {
                      context.toast(ToastType.success, '${selectedItems.length}개의 항목이 삭제되었어요');
                    }

                    onExitSelectionMode();
                  } catch (_) {
                    if (context.mounted) {
                      context.toast(ToastType.error, '삭제 중 오류가 발생했습니다');
                    }
                  }
                },
              ),
            );
          },
        ),
      ],
    );
  }
}

class MultiEntitiesMenuHeader extends StatelessWidget {
  const MultiEntitiesMenuHeader({
    super.key,
    required this.selectedCount,
    required this.folderCount,
    required this.postCount,
    required this.canvasCount,
  });

  final int selectedCount;
  final int folderCount;
  final int postCount;
  final int canvasCount;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          spacing: 16,
          children: [
            const Icon(LucideLightIcons.square_check, size: 20),
            Text('$selectedCount개 선택됨', style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600)),
          ],
        ),
        if (folderCount > 0 || postCount > 0 || canvasCount > 0)
          Padding(
            padding: const EdgeInsets.only(left: 36, top: 4),
            child: Row(
              spacing: 12,
              children: [
                if (folderCount > 0)
                  Row(
                    spacing: 4,
                    children: [
                      Icon(LucideLightIcons.folder, size: 14, color: context.colors.textSubtle),
                      Text('$folderCount개', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                    ],
                  ),
                if (postCount > 0)
                  Row(
                    spacing: 4,
                    children: [
                      Icon(LucideLightIcons.file, size: 14, color: context.colors.textSubtle),
                      Text('$postCount개', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                    ],
                  ),
                if (canvasCount > 0)
                  Row(
                    spacing: 4,
                    children: [
                      Icon(LucideLightIcons.line_squiggle, size: 14, color: context.colors.textSubtle),
                      Text('$canvasCount개', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                    ],
                  ),
              ],
            ),
          ),
      ],
    );
  }
}
