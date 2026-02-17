import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorFontFamilyTextOptionsToolbar extends HookWidget {
  const NativeEditorFontFamilyTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final attrs = useValueListenable(scope.attrs);

    final fontFamilyAttr = findAttr(attrs, 'font_family');
    final fontFamilyValues = (fontFamilyAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeValue = fontFamilyValues.length == 1
        ? fontFamilyValues[0]
        : (fontFamilyValues.isEmpty ? editorDefaultValues['fontFamily'] : null);

    final fontWeightAttr = findAttr(attrs, 'font_weight');
    final fontWeightValues = (fontWeightAttr?['values'] as List?)?.whereType<int>().toList() ?? [];
    final currentWeight = fontWeightValues.length == 1
        ? fontWeightValues[0]
        : (editorDefaultValues['fontWeight'] as int);

    final fontFamilies = scope.controller.fontManager?.fontFamilies ?? [];
    final activeFamilies = fontFamilies.where((f) => f.state == 'ACTIVE').toList();
    final items = activeFamilies.toList();
    if (activeValue != null && !items.any((f) => f.familyName == activeValue)) {
      final current = fontFamilies.where((f) => f.familyName == activeValue).firstOrNull;
      if (current != null) {
        items.add(current);
      }
    }
    final allItems = items
        .map(
          (f) => {
            'label': f.displayName,
            'value': f.familyName,
            'weights': f.fonts.where((font) => font.state == 'ACTIVE').map((font) => font.weight).toList(),
          },
        )
        .toList();

    return NativeEditorTextOptionsToolbar(
      items: allItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () {
            final weights = (item['weights'] as List?)?.cast<int>() ?? [];
            final defaultWeight =
                _getClosestWeight(weights, currentWeight) ?? (editorDefaultValues['fontWeight'] as int);

            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'font_family', 'family': item['value']},
            });
            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'font_weight', 'weight': defaultWeight},
            });
          },
        );
      },
    );
  }

  int? _getClosestWeight(List<int> weights, int fontWeight) {
    if (weights.isEmpty) {
      return null;
    }

    if (weights.contains(fontWeight)) {
      return fontWeight;
    }

    final sorted = weights.toList()..sort();
    int closest = sorted[0];
    int minDiff = (fontWeight - sorted[0]).abs();

    for (final weight in sorted) {
      final diff = (fontWeight - weight).abs();
      if (diff <= minDiff) {
        minDiff = diff;
        closest = weight;
      }
    }

    return closest;
  }
}
