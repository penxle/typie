import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
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
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/entity/__generated__/delete_entities_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/entity_fragment.data.gql.dart';
import 'package:typie/screens/entity/move_entity_modal.dart';
import 'package:typie/widgets/tappable.dart';

class SelectedEntitiesBar extends HookWidget {
  const SelectedEntitiesBar({
    super.key,
    required this.selectedItems,
    required this.entities,
    required this.onClearSelection,
    required this.onExitSelectionMode,
    required this.isVisible,
  });

  final Set<String> selectedItems;
  final List<GEntityScreen_Entity_entity> entities;
  final VoidCallback onClearSelection;
  final VoidCallback onExitSelectionMode;
  final bool isVisible;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();

    final folderCount = entities.where((e) => selectedItems.contains(e.id) && e.node.G__typename == 'Folder').length;
    final postCount = entities.where((e) => selectedItems.contains(e.id) && e.node.G__typename == 'Post').length;
    final canvasCount = entities.where((e) => selectedItems.contains(e.id) && e.node.G__typename == 'Canvas').length;

    return AnimatedPositioned(
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeInOut,
      left: 0,
      right: 0,
      bottom: isVisible ? 20 : 10,
      child: AnimatedOpacity(
        opacity: isVisible ? 1.0 : 0.0,
        duration: const Duration(milliseconds: 200),
        child: Center(
          child: IntrinsicWidth(
            child: Container(
              decoration: BoxDecoration(
                color: context.colors.surfaceDefault,
                border: Border.all(color: context.colors.borderStrong),
                borderRadius: const BorderRadius.all(Radius.circular(12)),
                boxShadow: [
                  BoxShadow(color: Colors.black.withValues(alpha: 0.08), blurRadius: 8, offset: const Offset(0, 2)),
                ],
              ),
              padding: const Pad(vertical: 8, left: 18, right: 12),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    '${selectedItems.length}개 선택됨',
                    style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w500),
                  ),
                  const SizedBox(width: 8),
                  Tappable(
                    onTap: onClearSelection,
                    child: Container(
                      padding: const Pad(all: 6),
                      decoration: BoxDecoration(
                        border: Border.all(color: context.colors.borderStrong),
                        borderRadius: const BorderRadius.all(Radius.circular(6)),
                      ),
                      child: const Icon(LucideLightIcons.x, size: 20),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(width: 1, height: 32, color: context.colors.borderStrong),
                  const SizedBox(width: 8),
                  Tappable(
                    onTap: () async {
                      await context.showBottomSheet(
                        child: BottomMenu(
                          header: _BottomMenuHeader(
                            selectedCount: selectedItems.length,
                            folderCount: folderCount,
                            postCount: postCount,
                            canvasCount: canvasCount,
                          ),
                          items: [
                            BottomMenuItem(
                              icon: LucideLightIcons.folder_symlink,
                              label: '다른 폴더로 옮기기',
                              onTap: () async {
                                unawaited(
                                  mixpanel.track(
                                    'move_entities_try',
                                    properties: {'totalCount': selectedItems.length, 'via': 'selected_entities_bar'},
                                  ),
                                );
                                await context.showBottomSheet(
                                  intercept: true,
                                  child: MoveEntityModal.multiple(
                                    entities: entities.where((e) => selectedItems.contains(e.id)).toList(),
                                    via: 'selected_entities_bar',
                                    onMoved: onExitSelectionMode,
                                  ),
                                );
                              },
                            ),
                            BottomMenuItem(
                              icon: LucideLightIcons.trash,
                              label: '삭제하기',
                              onTap: () async {
                                await context.showModal(
                                  child: ConfirmModal(
                                    title: '선택한 항목 삭제',
                                    message: '선택한 ${selectedItems.length}개 항목을 삭제하시겠어요?',
                                    confirmText: '삭제하기',
                                    confirmTextColor: context.colors.textBright,
                                    confirmBackgroundColor: context.colors.accentDanger,
                                    onConfirm: () async {
                                      try {
                                        await client.request(
                                          GEntityScreen_DeleteEntities_MutationReq(
                                            (b) => b..vars.input.entityIds.addAll(selectedItems),
                                          ),
                                        );

                                        unawaited(
                                          mixpanel.track(
                                            'delete_entities',
                                            properties: {
                                              'totalCount': selectedItems.length,
                                              'via': 'selected_entities_bar',
                                            },
                                          ),
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
                        ),
                      );
                    },
                    child: Container(
                      padding: const Pad(all: 6),
                      decoration: BoxDecoration(
                        border: Border.all(color: context.colors.borderStrong),
                        borderRadius: const BorderRadius.all(Radius.circular(6)),
                      ),
                      child: const Icon(LucideLightIcons.ellipsis_vertical, size: 20),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _BottomMenuHeader extends StatelessWidget {
  const _BottomMenuHeader({
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
            padding: const Pad(left: 36, top: 4),
            child: Row(
              spacing: 12,
              children: [
                if (folderCount > 0)
                  Row(
                    spacing: 4,
                    children: [
                      Icon(TypieIcons.folder_filled, size: 14, color: context.colors.textSubtle),
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
