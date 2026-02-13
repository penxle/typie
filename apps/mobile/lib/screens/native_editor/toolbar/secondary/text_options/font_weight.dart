import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorFontWeightTextOptionsToolbar extends HookWidget {
  const NativeEditorFontWeightTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final attrs = useValueListenable(scope.attrs);

    final fontFamilyAttr = findAttr(attrs, 'font_family');
    final fontFamilyValues = (fontFamilyAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final currentFontFamily = fontFamilyValues.length == 1
        ? fontFamilyValues[0]
        : editorDefaultValues['fontFamily'] as String;

    final fontWeightAttr = findAttr(attrs, 'font_weight');
    final fontWeightValues = (fontWeightAttr?['values'] as List?)?.whereType<int>().toList() ?? [];
    final activeValue = fontWeightValues.length == 1
        ? fontWeightValues[0]
        : (fontWeightValues.isEmpty ? editorDefaultValues['fontWeight'] as int : null);

    final currentFontFamilyAndWeights = _getCurrentFontFamilyAndWeights(currentFontFamily);

    final availableWeightItems = editorValues['fontWeight']!
        .where((item) => currentFontFamilyAndWeights.weights.contains(item['value'] as int))
        .toList();

    return NativeEditorTextOptionsToolbar(
      items: availableWeightItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () {
            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'font_weight', 'weight': item['value']},
            });
          },
        );
      },
    );
  }

  ({String family, List<int> weights}) _getCurrentFontFamilyAndWeights(String? fontFamilyOrId) {
    final defaultFontFamilyAndWeights = (
      family: editorDefaultValues['fontFamily'] as String,
      weights:
          (editorValues['fontFamily']!.firstWhere((f) => f['value'] == editorDefaultValues['fontFamily'])['weights']
                  as List)
              .cast<int>()
              .toList()
            ..sort(),
    );

    if (fontFamilyOrId == null) {
      return defaultFontFamilyAndWeights;
    }

    final systemFont = editorValues['fontFamily']!.firstWhereOrNull((f) => f['value'] == fontFamilyOrId);
    if (systemFont != null) {
      return (
        family: systemFont['value'] as String,
        weights: ((systemFont['weights'] as List?)?.cast<int>() ?? [])..sort(),
      );
    }

    return defaultFontFamilyAndWeights;
  }
}
