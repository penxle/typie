import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/icon.dart';

class TextOptionsToolbar extends HookWidget {
  const TextOptionsToolbar({
    required this.items,
    required this.activeValue,
    required this.builder,
    this.valueKey = 'value',
    super.key,
  });

  final List<Map<String, dynamic>> items;
  final dynamic activeValue;
  final Widget Function(BuildContext context, Map<String, dynamic> item, bool isActive) builder;
  final String valueKey;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);

    final controller = useScrollController();
    final key = useMemoized(GlobalKey.new);
    final keys = useMemoized(() => List.generate(items.length, (_) => GlobalKey()), [items]);

    useAsyncEffect(() async {
      final index = items.indexWhere((e) => e[valueKey] == activeValue);
      if (index == -1) {
        return null;
      }

      final listBox = key.currentContext?.findRenderObject() as RenderBox?;
      final itemBox = keys[index].currentContext?.findRenderObject() as RenderBox?;
      if (listBox == null || itemBox == null) {
        return null;
      }

      var currentOffset = 0.0;
      var parent = listBox.parent;

      while (parent != null) {
        if (parent is RenderBox && parent.parentData is StackParentData) {
          final left = (parent.parentData! as StackParentData).left;
          if (left != null) {
            currentOffset = left;
            break;
          }
        }

        parent = parent.parent;
      }

      final listOffset = listBox.localToGlobal(Offset.zero).dx - currentOffset;
      final itemOffset = itemBox.localToGlobal(Offset.zero, ancestor: listBox).dx;
      final itemCenter = itemBox.size.width / 2;
      final screenCenter = MediaQuery.sizeOf(context).width / 2;

      final targetOffset = (controller.offset + listOffset + itemOffset + itemCenter - screenCenter).clamp(
        controller.position.minScrollExtent,
        controller.position.maxScrollExtent,
      );

      await controller.animateTo(targetOffset, duration: const Duration(milliseconds: 150), curve: Curves.easeOut);

      return null;
    }, [activeValue]);

    return Row(
      children: [
        const Gap(4),
        IconToolbarButton(
          icon: LucideLightIcons.chevron_left,
          onTap: () {
            scope.secondaryToolbarMode.value = SecondaryToolbarMode.text;
          },
        ),
        const Gap(12),
        Expanded(
          child: SingleChildScrollView(
            key: key,
            controller: controller,
            scrollDirection: Axis.horizontal,
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const Pad(right: 16),
            child: Row(
              spacing: 4,
              children: items.asMap().entries.map((entry) {
                final index = entry.key;
                final item = entry.value;
                final isActive = item[valueKey] == activeValue;

                return KeyedSubtree(key: keys[index], child: builder(context, item, isActive));
              }).toList(),
            ),
          ),
        ),
      ],
    );
  }
}
