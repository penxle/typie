import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/widgets/tappable.dart';

class PasteOptionBottomSheet extends HookWidget {
  const PasteOptionBottomSheet({required this.onConfirm, super.key});

  final Future<void> Function(String mode) onConfirm;

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final rememberChoice = useState(false);
    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '붙여넣기 옵션',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Text('텍스트를 어떤 형식으로 붙여넣을까요?', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
          const Gap(16),
          GestureDetector(
            onTap: () {
              rememberChoice.value = !rememberChoice.value;
            },
            behavior: HitTestBehavior.opaque,
            child: Row(
              children: [
                Icon(
                  rememberChoice.value ? LucideLightIcons.square_check : LucideLightIcons.square,
                  size: 20,
                  color: rememberChoice.value ? context.colors.textDefault : context.colors.textSubtle,
                ),
                const Gap(4),
                Expanded(
                  child: Row(
                    children: [
                      Text('이 선택 기억하기', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                      const Gap(4),
                      Text('(설정 > 에디터에서 변경 가능)', style: TextStyle(fontSize: 12, color: context.colors.textFaint)),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const Gap(16),
          Tappable(
            onTap: () async {
              if (rememberChoice.value) {
                pref.pasteMode = 'html';
              }
              await onConfirm('html');
              if (context.mounted) {
                await context.router.root.maybePop();
              }
            },
            child: Container(
              decoration: BoxDecoration(
                border: Border.all(color: context.colors.borderDefault),
                borderRadius: BorderRadius.circular(8),
              ),
              padding: const Pad(horizontal: 12, vertical: 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('원본 서식 유지', style: TextStyle(fontSize: 16)),
                  const Gap(2),
                  Text('복사한 텍스트의 서식을 그대로 유지해요.', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
                ],
              ),
            ),
          ),
          const Gap(8),
          Tappable(
            onTap: () async {
              if (rememberChoice.value) {
                pref.pasteMode = 'text';
              }
              await onConfirm('text');
              if (context.mounted) {
                await context.router.root.maybePop();
              }
            },
            child: Container(
              decoration: BoxDecoration(
                border: Border.all(color: context.colors.borderDefault),
                borderRadius: BorderRadius.circular(8),
              ),
              padding: const Pad(horizontal: 12, vertical: 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('문서 서식 적용', style: TextStyle(fontSize: 16)),
                  const Gap(2),
                  Text('현재 문서의 서식을 적용하여 붙여넣어요.', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
