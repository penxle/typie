import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorFontFamilyTextOptionsToolbar extends HookWidget {
  const NativeEditorFontFamilyTextOptionsToolbar({super.key});

  int? _getDefaultWeight(String fontFamilyOrId, int fontWeight) {
    List<int> weights = [];

    final systemFont = editorValues['fontFamily']!.firstWhereOrNull((f) => f['value'] == fontFamilyOrId);
    if (systemFont != null) {
      weights = (systemFont['weights'] as List?)?.cast<int>() ?? []
        ..sort();
    }

    if (weights.isEmpty) {
      return null;
    }

    if (weights.contains(fontWeight)) {
      return fontWeight;
    }

    int closest = weights[0];
    int minDiff = (fontWeight - weights[0]).abs();

    for (final weight in weights) {
      final diff = (fontWeight - weight).abs();
      if (diff < minDiff) {
        minDiff = diff;
        closest = weight;
      }
    }

    return closest;
  }

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

    final allItems = editorValues['fontFamily']!;

    return NativeEditorTextOptionsToolbar(
      items: allItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () {
            final defaultWeight =
                _getDefaultWeight(item['value'] as String, currentWeight) ?? (editorDefaultValues['fontWeight'] as int);

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
}
