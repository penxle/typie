import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/widgets/tappable.dart';

enum EditorVersion { v1, v2 }

class EditorSelectBottomSheet extends StatelessWidget {
  const EditorSelectBottomSheet({required this.onSelect, super.key});

  final void Function(EditorVersion) onSelect;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text('에디터 선택', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          const Gap(8),
          Text(
            '어떤 에디터로 문서를 작성하시겠어요?\n한번 생성한 문서의 에디터는 변경할 수 없어요.',
            style: TextStyle(fontSize: 14, color: context.colors.textFaint),
          ),
          const Gap(20),
          _EditorOption(
            title: 'v1 에디터',
            description: '기존에 익숙하게 사용하시던 에디터에요.\n대부분의 경우 이 에디터가 적합해요.',
            onTap: () {
              context.router.pop();
              onSelect(EditorVersion.v1);
            },
          ),
          const Gap(12),
          _EditorOption(
            title: 'v2 에디터',
            description: '타이피 팀에서 새롭게 준비중인 에디터에요.\n아직 모든 기능 개발이 완료되지 않아, 실사용에는 적합하지 않아요.',
            onTap: () {
              context.router.pop();
              onSelect(EditorVersion.v2);
            },
          ),
          const Gap(20),
          Tappable(
            onTap: () async {
              await context.router.maybePop();
            },
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(8)),
              padding: const Pad(vertical: 16),
              child: const Text('취소', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
            ),
          ),
        ],
      ),
    );
  }
}

class _EditorOption extends StatelessWidget {
  const _EditorOption({required this.title, required this.description, required this.onTap});

  final String title;
  final String description;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        padding: const Pad(all: 16),
        decoration: BoxDecoration(
          color: context.colors.surfaceSubtle,
          border: Border.all(color: context.colors.borderDefault),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
            const Gap(4),
            Text(description, style: TextStyle(fontSize: 13, color: context.colors.textSubtle)),
          ],
        ),
      ),
    );
  }
}
