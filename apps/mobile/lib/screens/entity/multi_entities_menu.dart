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
import 'package:typie/widgets/popover/list.dart';
import 'package:typie/widgets/popover/popover.dart';

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
    final actions = _buildMultiEntitiesMenuActions(
      context: context,
      client: client,
      mixpanel: mixpanel,
      selectedItems: selectedItems,
      entities: entities,
      onExitSelectionMode: onExitSelectionMode,
      via: via,
    );

    final selectedEntities = entities.where((e) => selectedItems.contains(e.id)).toList();
    final folderCount = selectedEntities.where((e) => e.node.G__typename == 'Folder').length;
    final documentCount = selectedEntities.where((e) => e.node.G__typename == 'Document').length;

    return BottomMenu(
      header: MultiEntitiesMenuHeader(
        selectedCount: selectedItems.length,
        folderCount: folderCount,
        documentCount: documentCount,
      ),
      items: [
        for (final action in actions)
          BottomMenuItem(
            icon: action.icon,
            label: action.label,
            iconColor: action.iconColor,
            labelColor: action.labelColor,
            onTap: () {
              unawaited(action.onSelected());
            },
          ),
      ],
    );
  }
}

class MultiEntitiesPopoverPane extends HookWidget {
  const MultiEntitiesPopoverPane({
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
    final actions = _buildMultiEntitiesMenuActions(
      context: context,
      client: client,
      mixpanel: mixpanel,
      selectedItems: selectedItems,
      entities: entities,
      onExitSelectionMode: onExitSelectionMode,
      via: via,
    );

    return IntrinsicWidth(
      child: Padding(
        padding: const EdgeInsets.all(Popover.panePadding),
        child: PopoverList(
          indicatorColor: context.colors.surfaceMuted,
          items: [
            for (final action in actions)
              PopoverListItem(
                onSelected: () {
                  Popover.close(context);
                  unawaited(action.onSelected());
                },
                child: _MultiEntitiesMenuItem(
                  icon: action.icon,
                  label: action.label,
                  iconColor: action.iconColor,
                  labelColor: action.labelColor,
                ),
              ),
          ],
        ),
      ),
    );
  }
}

List<_MultiEntitiesMenuAction> _buildMultiEntitiesMenuActions({
  required BuildContext context,
  required GraphQLClient client,
  required Mixpanel mixpanel,
  required Set<String> selectedItems,
  required List<GEntityScreen_Entity_entity> entities,
  required VoidCallback onExitSelectionMode,
  required String via,
}) {
  final selectedEntities = entities.where((e) => selectedItems.contains(e.id)).toList();
  final folderIds = selectedEntities.where((e) => e.node.G__typename == 'Folder').map((e) => e.id).toList();
  final documentIds = selectedEntities.where((e) => e.node.G__typename == 'Document').map((e) => e.id).toList();

  Future<void> openFolderShare() async {
    unawaited(
      mixpanel.track('open_folder_share_modal', properties: {'via': 'multi_entities_menu', 'count': folderIds.length}),
    );
    await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: folderIds));
  }

  Future<void> openDocumentShare() async {
    unawaited(
      mixpanel.track(
        'open_document_share_modal',
        properties: {'via': 'multi_entities_menu', 'count': documentIds.length},
      ),
    );
    await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: documentIds));
  }

  Future<void> moveSelectedEntities() async {
    unawaited(mixpanel.track('move_entities_try', properties: {'totalCount': selectedItems.length, 'via': via}));
    await context.showBottomSheet(
      intercept: true,
      child: MoveEntityModal.multiple(entities: selectedEntities, via: via, onMoved: onExitSelectionMode),
    );
  }

  Future<void> deleteSelectedEntities() async {
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

            unawaited(mixpanel.track('delete_entities', properties: {'totalCount': selectedItems.length, 'via': via}));

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
  }

  return [
    if (folderIds.isNotEmpty)
      _MultiEntitiesMenuAction(
        icon: LucideLightIcons.blend,
        label: '폴더 ${folderIds.length}개 공유 및 게시',
        onSelected: openFolderShare,
      ),
    if (documentIds.isNotEmpty)
      _MultiEntitiesMenuAction(
        icon: LucideLightIcons.blend,
        label: '문서 ${documentIds.length}개 공유 및 게시',
        onSelected: openDocumentShare,
      ),
    _MultiEntitiesMenuAction(
      icon: LucideLightIcons.folder_symlink,
      label: '다른 폴더로 옮기기',
      onSelected: moveSelectedEntities,
    ),
    _MultiEntitiesMenuAction(
      icon: LucideLightIcons.trash_2,
      label: '삭제하기',
      onSelected: deleteSelectedEntities,
      iconColor: context.colors.textDanger,
      labelColor: context.colors.textDanger,
    ),
  ];
}

class MultiEntitiesMenuHeader extends StatelessWidget {
  const MultiEntitiesMenuHeader({
    super.key,
    required this.selectedCount,
    required this.folderCount,
    required this.documentCount,
  });

  final int selectedCount;
  final int folderCount;
  final int documentCount;

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
        if (folderCount > 0 || documentCount > 0)
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
                if (documentCount > 0)
                  Row(
                    spacing: 4,
                    children: [
                      Icon(LucideLightIcons.file, size: 14, color: context.colors.textSubtle),
                      Text('$documentCount개', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                    ],
                  ),
              ],
            ),
          ),
      ],
    );
  }
}

class _MultiEntitiesMenuAction {
  const _MultiEntitiesMenuAction({
    required this.icon,
    required this.label,
    required this.onSelected,
    this.iconColor,
    this.labelColor,
  });

  final IconData icon;
  final String label;
  final Future<void> Function() onSelected;
  final Color? iconColor;
  final Color? labelColor;
}

class _MultiEntitiesMenuItem extends StatelessWidget {
  const _MultiEntitiesMenuItem({required this.icon, required this.label, this.iconColor, this.labelColor});

  final IconData icon;
  final String label;
  final Color? iconColor;
  final Color? labelColor;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 42,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          spacing: 12,
          children: [
            Icon(icon, size: 18, color: iconColor ?? context.colors.textDefault),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w500,
                  color: labelColor ?? context.colors.textDefault,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
