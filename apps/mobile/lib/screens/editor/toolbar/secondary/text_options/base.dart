import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
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
    final keys = useMemoized(() => List.generate(items.length, (_) => GlobalKey()), [items]);
    final scope = EditorStateScope.of(context);

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        final index = items.indexWhere((e) => e[valueKey] == activeValue);
        if (index != -1 && keys[index].currentContext != null) {
          await Scrollable.ensureVisible(
            keys[index].currentContext!,
            alignment: 0.45,
            duration: const Duration(milliseconds: 150),
          );
        }
      });
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
