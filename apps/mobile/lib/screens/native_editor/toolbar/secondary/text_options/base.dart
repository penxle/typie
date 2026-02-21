import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/base.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class NativeEditorTextOptionsToolbar extends HookWidget {
  const NativeEditorTextOptionsToolbar({
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
    final scope = NativeEditorToolbarScope.of(context);
    final screenWidth = MediaQuery.sizeOf(context).width;
    final horizontalInset = screenWidth > 600 ? (screenWidth - 600) / 2 : 0.0;
    const backButtonHorizontalOffset = 4.0;
    const backButtonSize = 36.0;
    const backButtonGap = 12.0;
    final contentLeftPadding = horizontalInset + backButtonHorizontalOffset + backButtonSize + backButtonGap;

    final controller = useScrollController();
    final key = useMemoized(GlobalKey.new);
    final keys = useMemoized(() => List.generate(items.length, (_) => GlobalKey()), [items]);
    final lastMaxScrollExtent = useRef<double>(0);

    void scrollToActiveItem() {
      final index = items.indexWhere((e) => e[valueKey] == activeValue);
      if (index == -1) {
        return;
      }

      final listBox = key.currentContext?.findRenderObject() as RenderBox?;
      final itemBox = keys[index].currentContext?.findRenderObject() as RenderBox?;
      if (listBox == null || itemBox == null) {
        return;
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

      unawaited(controller.animateTo(targetOffset, duration: const Duration(milliseconds: 150), curve: Curves.easeOut));
    }

    useAsyncEffect(() async {
      scrollToActiveItem();
      return null;
    }, [activeValue]);

    return Stack(
      clipBehavior: Clip.none,
      children: [
        Positioned.fill(
          child: NotificationListener<ScrollMetricsNotification>(
            onNotification: (notification) {
              final maxExtent = notification.metrics.maxScrollExtent;
              if (lastMaxScrollExtent.value != maxExtent) {
                lastMaxScrollExtent.value = maxExtent;
                scrollToActiveItem();
              }
              return false;
            },
            child: SingleChildScrollView(
              key: key,
              controller: controller,
              scrollDirection: Axis.horizontal,
              physics: const AlwaysScrollableScrollPhysics(),
              clipBehavior: Clip.none,
              padding: EdgeInsets.only(left: contentLeftPadding, right: horizontalInset + 16),
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
        ),
        Positioned(
          left: horizontalInset + backButtonHorizontalOffset,
          top: 0,
          bottom: 0,
          child: Align(
            alignment: Alignment.centerLeft,
            child: ToolbarButton(
              onTap: () {
                scope.secondaryToolbarMode.value = SecondaryToolbarMode.text;
              },
              builder: (context, color, _) {
                return Container(
                  decoration: BoxDecoration(
                    color: context.colors.surfaceDefault.withValues(alpha: 0.72),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  padding: const EdgeInsets.all(8),
                  child: Icon(LucideLightIcons.chevron_left, size: 20, color: color),
                );
              },
            ),
          ),
        ),
      ],
    );
  }
}
