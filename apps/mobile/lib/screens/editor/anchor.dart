import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/tappable.dart';

class AnchorBottomSheet extends HookWidget {
  const AnchorBottomSheet({required this.scope, super.key});

  final EditorStateScope scope;

  @override
  Widget build(BuildContext context) {
    final anchors = useState<List<Map<String, dynamic>>>([]);
    final currentNode = useState<Map<String, dynamic>?>(null);
    final isLoading = useState(true);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);
    final webViewController = useValueListenable(scope.webViewController);
    final scrollController = useScrollController();

    final removedAnchors = useState<List<String>>([]);
    final hasInitiallyScrolled = useState(false);

    useAsyncEffect(() async {
      try {
        final results = await Future.wait([
          webViewController?.callProcedure('getAnchorPositionsV2') ?? Future<dynamic>.value(),
          webViewController?.callProcedure('getCurrentNodeV2') ?? Future<dynamic>.value(),
        ]);

        if (results[0] != null) {
          anchors.value = (results[0] as List<dynamic>).cast<Map<String, dynamic>>();
        }

        if (results[1] != null) {
          currentNode.value = results[1] as Map<String, dynamic>;
        }
      } finally {
        isLoading.value = false;
      }

      return null;
    }, [webViewController]);

    useAsyncEffect(() async {
      currentNode.value = await webViewController?.callProcedure('getCurrentNodeV2') as Map<String, dynamic>?;

      return null;
    }, [proseMirrorState?.currentNode]);

    final bottomPadding = MediaQuery.paddingOf(context).bottom;

    // currentNode와 anchors를 합쳐서 position으로 정렬
    final allNodes = useMemoized(() {
      final nodes = <Map<String, dynamic>>[];

      // anchors 추가
      for (final anchor in anchors.value) {
        nodes.add({
          ...anchor,
          'isAnchor': true,
          'isCurrent': currentNode.value != null && anchor['nodeId'] == currentNode.value!['nodeId'],
          'isSpecial': false,
        });
      }

      // currentNode 추가 (anchors에 없는 경우만)
      if (currentNode.value != null && !anchors.value.any((a) => a['nodeId'] == currentNode.value!['nodeId'])) {
        nodes.add({...currentNode.value!, 'isAnchor': false, 'isCurrent': true, 'isSpecial': false});
      }

      // position으로 정렬 (최상단, 최하단 제외)
      final middleNodes = nodes.where((node) => node['nodeId'] != 'top').toList()
        ..sort((a, b) => ((a['position'] as num?) ?? 0).compareTo((b['position'] as num?) ?? 0));

      nodes
        ..clear()
        ..add({
          'nodeId': 'top',
          'name': '첫 줄로 가기',
          'excerpt': '',
          'position': 0,
          'isAnchor': false,
          'isCurrent': false,
          'isSpecial': true,
          'icon': LucideLightIcons.arrow_up_to_line,
        })
        ..addAll(middleNodes)
        ..add({
          'nodeId': 'bottom',
          'name': '마지막 줄로 가기',
          'excerpt': '',
          'position': 1,
          'isAnchor': false,
          'isCurrent': false,
          'isSpecial': true,
          'icon': LucideLightIcons.arrow_down_to_line,
        });

      return nodes;
    }, [anchors.value, currentNode.value]);

    // currentNode로 스크롤 (처음 열 때만)
    useEffect(() {
      if (!isLoading.value && currentNode.value != null && allNodes.isNotEmpty && !hasInitiallyScrolled.value) {
        final currentIndex = allNodes.indexWhere((node) => node['nodeId'] == currentNode.value!['nodeId']);

        if (currentIndex != -1) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (scrollController.hasClients) {
              // 아이템 높이(52) + 간격(8)
              const itemHeight = 60.0;
              final viewportHeight = scrollController.position.viewportDimension;
              final scrollableHeight = scrollController.position.maxScrollExtent;

              // 현재 아이템을 뷰포트 중앙에 위치시키기
              final itemOffset = currentIndex * itemHeight;
              final targetOffset = itemOffset - (viewportHeight / 2) + (52 / 2); // 52는 아이템의 실제 높이

              // 스크롤 가능한 범위 내에서 클램프
              final clampedOffset = targetOffset.clamp(0.0, scrollableHeight);

              scrollController.jumpTo(clampedOffset);
              hasInitiallyScrolled.value = true;
            }
          });
        }
      }
      return null;
    }, [isLoading.value, currentNode.value, allNodes]);

    Future<void> editAnchorName(String nodeId, String currentName) async {
      String? newName;

      await context.showModal(
        intercept: true,
        child: HookForm(
          onSubmit: (form) async {
            newName = form.data['name'] as String;
          },
          builder: (context, form) {
            return ConfirmModal(
              title: '북마크 편집',
              confirmText: '저장',
              onConfirm: () async {
                await form.submit();
              },
              child: HookFormTextField.collapsed(
                initialValue: currentName,
                name: 'name',
                placeholder: '북마크 이름',
                autofocus: true,
                style: const TextStyle(fontSize: 16),
                submitOnEnter: false,
                keyboardType: TextInputType.text,
                maxLength: 20,
              ),
            );
          },
        ),
      );

      if (newName != null && newName != currentName) {
        await webViewController?.callProcedure('updateAnchorName', {'nodeId': nodeId, 'name': newName});

        final updatedAnchors = await webViewController?.callProcedure('getAnchorPositionsV2');
        if (updatedAnchors != null) {
          anchors.value = (updatedAnchors as List<dynamic>).cast<Map<String, dynamic>>();
        }
      }
    }

    Future<void> toggleBookmark(String nodeId, bool isAnchor, bool isCurrent, String? name) async {
      if (!isAnchor || removedAnchors.value.contains(nodeId)) {
        await webViewController?.callProcedure('addAnchorWithName', {'nodeId': nodeId, 'name': name});

        removedAnchors.value = removedAnchors.value.where((id) => id != nodeId).toList();

        if (isCurrent && !isAnchor) {
          anchors.value = [...anchors.value, currentNode.value!];
        }
      } else {
        await webViewController?.callProcedure('removeAnchor', nodeId);
        removedAnchors.value = [...removedAnchors.value, nodeId];
      }
    }

    return AppBottomSheet(
      includeBottomPadding: false,
      padding: const Pad(horizontal: 20),
      child: isLoading.value
          ? Padding(
              padding: Pad(vertical: 20, bottom: bottomPadding + 12),
              child: const Center(child: CircularProgressIndicator()),
            )
          : ConstrainedBox(
              constraints: BoxConstraints(maxHeight: MediaQuery.of(context).size.height * 0.4),
              child: ListView.separated(
                controller: scrollController,
                shrinkWrap: true,
                padding: Pad(bottom: bottomPadding + 12),
                itemCount: allNodes.length,
                separatorBuilder: (context, index) => const SizedBox(height: 8),
                itemBuilder: (context, index) {
                  final node = allNodes[index];
                  final nodeId = node['nodeId'] as String;
                  final name = node['name'] as String?;
                  final excerpt = node['excerpt'] as String;
                  final position = node['position'] as num;
                  final isCurrent = node['isCurrent'] as bool;
                  final isAnchor = node['isAnchor'] as bool;
                  final isSpecial = node['isSpecial'] as bool? ?? false;
                  final isRemoved = removedAnchors.value.contains(nodeId);

                  return Tappable(
                    onTap: () async {
                      if (nodeId == 'top') {
                        await webViewController?.callProcedure('scrollToTop');
                      } else if (nodeId == 'bottom') {
                        await webViewController?.callProcedure('scrollToBottom');
                      } else {
                        await webViewController?.callProcedure('clickAnchor', nodeId);
                      }
                    },
                    child: Container(
                      height: 52,
                      decoration: BoxDecoration(
                        color: context.colors.surfaceDefault,
                        border: Border.all(color: context.colors.borderStrong),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Padding(
                        padding: const Pad(horizontal: 12),
                        child: Row(
                          children: [
                            if (isSpecial)
                              Container(
                                width: 40,
                                alignment: Alignment.center,
                                child: Icon(node['icon'] as IconData, size: 16, color: context.colors.textDefault),
                              )
                            else
                              Container(
                                constraints: const BoxConstraints(minWidth: 40),
                                alignment: Alignment.center,
                                child: Text(
                                  isCurrent ? '현재' : '${(position * 100).toStringAsFixed(0)}%',
                                  style: TextStyle(
                                    fontSize: 14,
                                    fontWeight: FontWeight.w600,
                                    color: context.colors.textFaint,
                                  ),
                                ),
                              ),
                            const Gap(8),
                            Expanded(
                              child: Text(
                                name ?? excerpt,
                                style: TextStyle(
                                  fontSize: 16,
                                  fontWeight: FontWeight.w500,
                                  color: context.colors.textDefault,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            if (!isSpecial) ...[
                              const Gap(12),
                              if (isAnchor && !isRemoved) ...[
                                Tappable(
                                  onTap: () => editAnchorName(nodeId, name ?? excerpt),
                                  child: Container(
                                    width: 36,
                                    height: 36,
                                    decoration: BoxDecoration(
                                      border: Border.all(color: context.colors.borderStrong),
                                      borderRadius: BorderRadius.circular(6),
                                    ),
                                    child: Center(
                                      child: Icon(LucideLightIcons.pen, size: 18, color: context.colors.textFaint),
                                    ),
                                  ),
                                ),
                                const Gap(8),
                              ],
                              Tappable(
                                onTap: () => toggleBookmark(nodeId, isAnchor, isCurrent, name),
                                child: Container(
                                  width: 36,
                                  height: 36,
                                  decoration: BoxDecoration(
                                    border: Border.all(color: context.colors.borderStrong),
                                    borderRadius: BorderRadius.circular(6),
                                  ),
                                  child: Center(
                                    child: Icon(
                                      (isAnchor && !isRemoved) ? TypieIcons.bookmark_filled : LucideLightIcons.plus,
                                      size: 18,
                                      color: (isAnchor && !isRemoved)
                                          ? context.theme.brightness == Brightness.dark
                                                ? const Color(0xFFB8860B)
                                                : const Color(0xFFFACC15)
                                          : context.colors.textFaint,
                                    ),
                                  ),
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
    );
  }
}
