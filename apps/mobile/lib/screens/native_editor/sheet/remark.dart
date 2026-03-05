import 'dart:async';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/__generated__/remark_user_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/remark_user_query.req.gql.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/widgets/tappable.dart';

const _remarkNodeLabels = {
  'image': '이미지',
  'file': '파일',
  'embed': '임베드',
  'archived': '보관된 블록',
  'horizontal_rule': '구분선',
  'blockquote': '인용구',
  'callout': '강조',
  'bullet_list': '순서 없는 목록',
  'ordered_list': '순서 있는 목록',
  'fold': '접기',
  'table': '표',
};

String _resolveRemarkNodeInfoText({
  required RemarkOverlayInfo remark,
  required Map<String, RemarkNodeContext> nodeContexts,
}) {
  final context = nodeContexts[remark.nodeId];
  if (context == null) {
    return '';
  }

  if (context.isTextblock) {
    final normalized = context.nodeText.replaceAll(RegExp(r'\s+'), ' ').trim();
    return normalized.isNotEmpty ? normalized : '빈 텍스트';
  }

  return _remarkNodeLabels[context.nodeType] ?? context.nodeType;
}

class RemarkBottomSheet extends HookWidget {
  const RemarkBottomSheet({required this.controller, required this.client, required this.userId, super.key});

  final EditorController controller;
  final GraphQLClient client;
  final String userId;

  @override
  Widget build(BuildContext context) {
    final isBlockTab = useState(true);
    final activeRemarkId = useState<String?>(null);
    final editingRemarkId = useState<String?>(null);
    final editController = useTextEditingController();
    final inputController = useTextEditingController();
    final inputFocusNode = useFocusNode();
    final listScrollController = useScrollController();
    final pendingScrollToBottomAfterAdd = useState(false);
    final pendingScrollToCurrentBlock = useState(false);
    final inputText = useState('');

    useEffect(() {
      void listener() {
        inputText.value = inputController.text;
      }

      inputController.addListener(listener);
      return () => inputController.removeListener(listener);
    }, [inputController]);

    final users = useState<Map<String, GNativeEditor_RemarkUser_QueryData_userView>>({});

    final state = useListenableSelector(controller, () => controller.state);
    final currentBlockOverlay = useValueListenable(controller.currentBlockOverlay);
    final remarks = state.remarks;
    final currentBlockNodeId = state.currentBlockNodeId;
    final remarkNodeContexts = useValueListenable(controller.remarkNodeContexts);

    useEffect(() {
      final uniqueUserIds = remarks.map((r) => r.userId).toSet();
      final currentUsers = users.value;
      final missingIds = uniqueUserIds.where((id) => !currentUsers.containsKey(id)).toList();

      if (missingIds.isEmpty) {
        return null;
      }

      Future<void> fetchAll() async {
        final newUsers = Map<String, GNativeEditor_RemarkUser_QueryData_userView>.from(currentUsers);
        for (final userId in missingIds) {
          try {
            final data = await client.request(GNativeEditor_RemarkUser_QueryReq((b) => b..vars.userId = userId));
            newUsers[userId] = data.userView;
          } catch (_) {}
        }
        users.value = newUsers;
      }

      unawaited(fetchAll());
      return null;
    }, [remarks]);

    final allRemarks = useMemoized(() {
      final sorted = [...remarks]
        ..sort((a, b) {
          final pageCmp = a.pageIdx.compareTo(b.pageIdx);
          if (pageCmp != 0) {
            return pageCmp;
          }
          return a.boundsY.compareTo(b.boundsY);
        });
      return sorted;
    }, [remarks]);

    final blockRemarks = useMemoized(
      () => currentBlockNodeId == null
          ? <RemarkOverlayInfo>[]
          : remarks.where((r) => r.nodeId == currentBlockNodeId).toList(),
      [remarks, currentBlockNodeId],
    );

    void addRemark() {
      final text = inputController.text.trim();
      if (text.isEmpty || currentBlockNodeId == null) {
        return;
      }

      controller.dispatch({
        'type': 'addRemark',
        'nodeId': currentBlockNodeId,
        'userId': userId,
        'text': text,
        'createdAt': DateTime.now().millisecondsSinceEpoch,
      });
      inputController.clear();
      inputFocusNode.requestFocus();
      pendingScrollToBottomAfterAdd.value = true;
    }

    void deleteRemark(RemarkOverlayInfo remark) {
      controller.dispatch({'type': 'removeRemark', 'nodeId': remark.nodeId, 'remarkId': remark.remarkId});
      if (activeRemarkId.value == remark.remarkId) {
        activeRemarkId.value = null;
      }
    }

    void saveEdit(RemarkOverlayInfo remark) {
      final trimmed = editController.text.trim();
      if (trimmed.isEmpty) {
        return;
      }

      controller.dispatch({
        'type': 'updateRemark',
        'nodeId': remark.nodeId,
        'remarkId': remark.remarkId,
        'text': trimmed,
      });
      editingRemarkId.value = null;
    }

    final currentRemarks = isBlockTab.value ? blockRemarks : allRemarks;
    final bottomPadding = MediaQuery.paddingOf(context).bottom;

    RemarkOverlayInfo? currentBlockHighlightTarget() {
      final nodeId = currentBlockNodeId;
      if (nodeId == null) {
        return null;
      }
      for (final remark in allRemarks) {
        if (remark.nodeId == nodeId) {
          return remark;
        }
      }
      final overlay = controller.currentBlockOverlay.value;
      if (overlay == null || overlay.nodeId != nodeId || overlay.pageIdx < 0) {
        return null;
      }
      return RemarkOverlayInfo(
        pageIdx: overlay.pageIdx,
        nodeId: nodeId,
        remarkId: '__current_block_$nodeId',
        userId: userId,
        text: '',
        createdAt: 0,
        boundsX: overlay.x,
        boundsY: overlay.y,
        boundsWidth: overlay.width,
        boundsHeight: overlay.height,
      );
    }

    useEffect(() {
      if (!pendingScrollToBottomAfterAdd.value || !isBlockTab.value) {
        return null;
      }

      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!listScrollController.hasClients) {
          return;
        }
        unawaited(
          listScrollController.animateTo(
            listScrollController.position.maxScrollExtent,
            duration: const Duration(milliseconds: 180),
            curve: Curves.easeOut,
          ),
        );
      });
      pendingScrollToBottomAfterAdd.value = false;
      return null;
    }, [pendingScrollToBottomAfterAdd.value, blockRemarks.length, isBlockTab.value]);

    useEffect(() {
      pendingScrollToCurrentBlock.value = true;
      return null;
    }, const []);

    useEffect(
      () {
        if (!pendingScrollToCurrentBlock.value) {
          return null;
        }
        if (!isBlockTab.value) {
          pendingScrollToCurrentBlock.value = false;
          return null;
        }

        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!context.mounted || !isBlockTab.value) {
            pendingScrollToCurrentBlock.value = false;
            return;
          }
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (!context.mounted || !isBlockTab.value) {
              pendingScrollToCurrentBlock.value = false;
              return;
            }
            final target = currentBlockHighlightTarget();
            if (target == null) {
              if (currentBlockNodeId == null) {
                controller.remarkHighlightTarget.value = null;
                controller.scrollIntoView();
                pendingScrollToCurrentBlock.value = false;
              }
              return;
            }
            controller.scrollToRemark(target);
            pendingScrollToCurrentBlock.value = false;
          });
        });
        return null;
      },
      [
        pendingScrollToCurrentBlock.value,
        isBlockTab.value,
        currentBlockNodeId,
        blockRemarks.length,
        currentBlockOverlay,
      ],
    );

    return AppBottomSheet(
      includeBottomPadding: false,
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                spacing: 6,
                children: [
                  Text(
                    '코멘트',
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700, color: context.colors.textDefault),
                  ),
                  if (currentRemarks.isNotEmpty)
                    Text(
                      '${currentRemarks.length}',
                      style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                    ),
                ],
              ),
              Container(
                padding: const Pad(all: 2),
                decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(8)),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _TabPill(
                      label: '전체',
                      isActive: !isBlockTab.value,
                      onTap: () {
                        isBlockTab.value = false;
                        pendingScrollToCurrentBlock.value = false;
                        activeRemarkId.value = null;
                        controller.remarkHighlightTarget.value = null;
                      },
                    ),
                    _TabPill(
                      label: '현재 위치',
                      isActive: isBlockTab.value,
                      onTap: () {
                        isBlockTab.value = true;
                        pendingScrollToCurrentBlock.value = true;
                      },
                    ),
                  ],
                ),
              ),
            ],
          ),
          const Gap(16),
          if (currentRemarks.isEmpty) ...[
            Padding(
              padding: Pad(vertical: 32, bottom: isBlockTab.value ? 0 : bottomPadding + 12),
              child: Column(
                children: [
                  Icon(LucideLightIcons.message_square_text, size: 24, color: context.colors.textFaint),
                  const Gap(8),
                  Text(
                    isBlockTab.value ? '이 위치에 코멘트가 없습니다' : '코멘트가 없습니다',
                    style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                  ),
                ],
              ),
            ),
          ] else ...[
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: MediaQuery.sizeOf(context).height * 0.4),
              child: SingleChildScrollView(
                controller: listScrollController,
                padding: Pad(bottom: isBlockTab.value ? 0 : bottomPadding + 12),
                child: Column(
                  children: currentRemarks
                      .map(
                        (remark) => _RemarkItem(
                          key: ValueKey(remark.remarkId),
                          remark: remark,
                          user: users.value[remark.userId],
                          isActive: activeRemarkId.value == remark.remarkId,
                          isEditing: editingRemarkId.value == remark.remarkId,
                          isOwnRemark: remark.userId == userId,
                          editController: editController,
                          nodeInfoText: isBlockTab.value
                              ? null
                              : _resolveRemarkNodeInfoText(remark: remark, nodeContexts: remarkNodeContexts),
                          onTap: () {
                            activeRemarkId.value = remark.remarkId;
                            controller.scrollToRemark(remark);
                          },
                          onStartEdit: () {
                            editController.text = remark.text;
                            editingRemarkId.value = remark.remarkId;
                          },
                          onSaveEdit: () => saveEdit(remark),
                          onCancelEdit: () => editingRemarkId.value = null,
                          onDelete: () async {
                            await context.showBottomSheet(
                              child: ConfirmBottomSheet(
                                title: '코멘트 삭제',
                                message: '코멘트를 삭제하시겠어요?',
                                confirmText: '삭제',
                                confirmTextColor: context.colors.textBright,
                                confirmBackgroundColor: context.colors.accentDanger,
                                onConfirm: () => deleteRemark(remark),
                              ),
                            );
                          },
                        ),
                      )
                      .toList(),
                ),
              ),
            ),
          ],
          if (isBlockTab.value && currentBlockNodeId != null) ...[
            Container(height: 1, color: context.colors.borderSubtle),
            const Gap(12),
            _RemarkInput(
              controller: inputController,
              focusNode: inputFocusNode,
              hasText: inputText.value.trim().isNotEmpty,
              onSubmit: addRemark,
            ),
            Gap(bottomPadding + 12),
          ],
        ],
      ),
    );
  }
}

class _TabPill extends StatelessWidget {
  const _TabPill({required this.label, required this.isActive, required this.onTap});

  final String label;
  final bool isActive;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        padding: const Pad(horizontal: 10, vertical: 5),
        decoration: BoxDecoration(
          color: isActive ? context.colors.surfaceDefault : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 12,
            fontWeight: isActive ? FontWeight.w600 : FontWeight.w500,
            color: isActive ? context.colors.textDefault : context.colors.textFaint,
          ),
        ),
      ),
    );
  }
}

class _RemarkItem extends StatelessWidget {
  const _RemarkItem({
    required this.remark,
    required this.user,
    required this.isActive,
    required this.isEditing,
    required this.isOwnRemark,
    required this.editController,
    this.nodeInfoText,
    required this.onTap,
    required this.onStartEdit,
    required this.onSaveEdit,
    required this.onCancelEdit,
    required this.onDelete,
    super.key,
  });

  final RemarkOverlayInfo remark;
  final GNativeEditor_RemarkUser_QueryData_userView? user;
  final bool isActive;
  final bool isEditing;
  final bool isOwnRemark;
  final TextEditingController editController;
  final String? nodeInfoText;
  final VoidCallback onTap;
  final VoidCallback onStartEdit;
  final VoidCallback onSaveEdit;
  final VoidCallback onCancelEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final timeText = Jiffy.parseFromMillisecondsSinceEpoch(remark.createdAt).ago;
    final dpr = MediaQuery.devicePixelRatioOf(context);
    final imageSize = pow(2, (log(28 * dpr) / log(2)).ceil()).toInt();
    final hasNodeInfoText = nodeInfoText != null && nodeInfoText!.isNotEmpty;

    return Tappable(
      onTap: onTap,
      child: Container(
        padding: const Pad(horizontal: 4, vertical: 10),
        decoration: BoxDecoration(
          color: isActive ? context.colors.surfaceMuted : Colors.transparent,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (user != null)
              ClipOval(
                child: CachedNetworkImage(
                  imageUrl: '${user!.avatar.url}?s=$imageSize&q=75',
                  width: 28,
                  height: 28,
                  fit: BoxFit.cover,
                  fadeInDuration: const Duration(milliseconds: 150),
                ),
              )
            else
              Container(
                width: 28,
                height: 28,
                decoration: BoxDecoration(color: context.colors.surfaceMuted, shape: BoxShape.circle),
              ),
            const Gap(10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Row(
                          children: [
                            if (user != null)
                              Flexible(
                                child: Text(
                                  user!.name,
                                  style: TextStyle(
                                    fontSize: 13,
                                    fontWeight: FontWeight.w600,
                                    color: context.colors.textDefault,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                  maxLines: 1,
                                ),
                              ),
                            if (user != null) const Gap(6),
                            Text(timeText, style: TextStyle(fontSize: 12, color: context.colors.textFaint)),
                          ],
                        ),
                      ),
                      if (hasNodeInfoText) ...[
                        const Gap(8),
                        ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 150),
                          child: Text(
                            nodeInfoText!,
                            textAlign: TextAlign.right,
                            style: TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.w500,
                              color: context.colors.textFaint,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ],
                  ),
                  const Gap(4),
                  if (isEditing) ...[
                    TextField(
                      controller: editController,
                      maxLines: null,
                      minLines: 2,
                      autofocus: true,
                      style: TextStyle(fontSize: 14, color: context.colors.textDefault),
                      decoration: InputDecoration(
                        isDense: true,
                        contentPadding: const Pad(all: 10),
                        filled: true,
                        fillColor: context.colors.surfaceDefault,
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: context.colors.borderDefault),
                        ),
                        focusedBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: BorderSide(color: context.colors.accentBrand),
                        ),
                      ),
                    ),
                    const Gap(8),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      spacing: 8,
                      children: [
                        Tappable(
                          onTap: onCancelEdit,
                          child: Container(
                            padding: const Pad(horizontal: 12, vertical: 6),
                            decoration: BoxDecoration(
                              border: Border.all(color: context.colors.borderDefault),
                              borderRadius: BorderRadius.circular(6),
                            ),
                            child: Text(
                              '취소',
                              style: TextStyle(
                                fontSize: 13,
                                fontWeight: FontWeight.w500,
                                color: context.colors.textSubtle,
                              ),
                            ),
                          ),
                        ),
                        Tappable(
                          onTap: onSaveEdit,
                          child: Container(
                            padding: const Pad(horizontal: 12, vertical: 6),
                            decoration: BoxDecoration(
                              color: context.colors.surfaceInverse,
                              borderRadius: BorderRadius.circular(6),
                            ),
                            child: Text(
                              '저장',
                              style: TextStyle(
                                fontSize: 13,
                                fontWeight: FontWeight.w500,
                                color: context.colors.textInverse,
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ] else ...[
                    Text(
                      remark.text,
                      style: TextStyle(fontSize: 14, height: 1.5, color: context.colors.textSubtle),
                      maxLines: isActive ? null : 3,
                      overflow: isActive ? null : TextOverflow.ellipsis,
                    ),
                    if (isOwnRemark && isActive) ...[
                      const Gap(8),
                      Row(
                        spacing: 16,
                        children: [
                          Tappable(
                            onTap: onStartEdit,
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              spacing: 4,
                              children: [
                                Icon(LucideLightIcons.pencil, size: 13, color: context.colors.textFaint),
                                Text(
                                  '수정',
                                  style: TextStyle(
                                    fontSize: 12,
                                    fontWeight: FontWeight.w500,
                                    color: context.colors.textFaint,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          Tappable(
                            onTap: onDelete,
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              spacing: 4,
                              children: [
                                Icon(LucideLightIcons.trash_2, size: 13, color: context.colors.textFaint),
                                Text(
                                  '삭제',
                                  style: TextStyle(
                                    fontSize: 12,
                                    fontWeight: FontWeight.w500,
                                    color: context.colors.textFaint,
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ],
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _RemarkInput extends StatelessWidget {
  const _RemarkInput({
    required this.controller,
    required this.focusNode,
    required this.hasText,
    required this.onSubmit,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool hasText;
  final VoidCallback onSubmit;

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: controller,
      focusNode: focusNode,
      style: TextStyle(fontSize: 14, color: context.colors.textDefault),
      decoration: InputDecoration(
        hintText: '코멘트 입력...',
        hintStyle: TextStyle(fontSize: 14, color: context.colors.textFaint),
        isDense: true,
        filled: true,
        fillColor: context.colors.surfaceMuted,
        contentPadding: const Pad(horizontal: 14, vertical: 10),
        border: OutlineInputBorder(borderRadius: BorderRadius.circular(20), borderSide: BorderSide.none),
        enabledBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(20), borderSide: BorderSide.none),
        focusedBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(20), borderSide: BorderSide.none),
        suffixIcon: Tappable(
          onTap: onSubmit,
          child: Padding(
            padding: const Pad(all: 6),
            child: Container(
              width: 28,
              height: 28,
              decoration: BoxDecoration(
                color: hasText ? context.colors.surfaceInverse : Colors.transparent,
                shape: BoxShape.circle,
              ),
              child: Icon(
                LucideLightIcons.arrow_up,
                size: 16,
                color: hasText ? context.colors.textInverse : context.colors.textFaint,
              ),
            ),
          ),
        ),
        suffixIconConstraints: const BoxConstraints(maxHeight: 40, maxWidth: 40),
      ),
      onSubmitted: (_) => onSubmit(),
    );
  }
}
