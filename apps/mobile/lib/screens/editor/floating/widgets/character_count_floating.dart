import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/floating/editor_floating_widget.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/services/preference.dart';

class CharacterCountFloating extends HookWidget {
  const CharacterCountFloating({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final characterCountState = useValueListenable(scope.characterCountState);
    final pref = useService<Pref>();

    final isExpanded = useState(false);

    if (characterCountState == null) {
      return const SizedBox.shrink();
    }

    final savedPosition = pref.characterCountFloatingPosition;
    final initialOffset = savedPosition != null
        ? Offset(savedPosition['x'] ?? 20, savedPosition['y'] ?? 20)
        : const Offset(20, 20);

    return EditorFloatingWidget(
      storageKey: 'character_count',
      initialOffset: initialOffset,
      isExpanded: isExpanded.value,
      onPositionChanged: (position) {
        pref.characterCountFloatingPosition = {'x': position.dx, 'y': position.dy};
      },
      onTap: () {
        isExpanded.value = !isExpanded.value;
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        decoration: BoxDecoration(
          color: context.colors.surfaceSubtle.withValues(alpha: 0.95),
          border: Border.all(color: context.colors.borderDefault),
          borderRadius: BorderRadius.circular(16),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(LucideLightIcons.type_, size: 14, color: context.colors.textSubtle),
                const SizedBox(width: 6),
                Text(
                  '${characterCountState.countWithWhitespace.comma}자',
                  style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: context.colors.textDefault),
                ),
                const SizedBox(width: 8),
                AnimatedRotation(
                  turns: isExpanded.value ? 0.25 : 0,
                  duration: const Duration(milliseconds: 200),
                  child: Icon(LucideLightIcons.chevron_right, size: 14, color: context.colors.textSubtle),
                ),
              ],
            ),
            if (isExpanded.value) ...[
              const SizedBox(height: 8),
              _buildCountRow(label: '공백 미포함', count: characterCountState.countWithoutWhitespace, context: context),
              const SizedBox(height: 4),
              _buildCountRow(
                label: '공백/부호 미포함',
                count: characterCountState.countWithoutWhitespaceAndPunctuation,
                context: context,
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildCountRow({required String label, required int count, required BuildContext context}) {
    return Text('$label: ${count.comma}자', style: TextStyle(fontSize: 13, color: context.colors.textSubtle));
  }
}
