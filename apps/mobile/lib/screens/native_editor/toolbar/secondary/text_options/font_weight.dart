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
    final uniformStyles = useValueListenable(scope.uniformStyles);
    final mixedStyles = useValueListenable(scope.mixedStyles);

    final fontFamilyStyle = uniformStyles.firstWhereOrNull((m) => m['type'] == 'font_family');
    final currentFontFamily = fontFamilyStyle?['family'] as String? ?? editorDefaultValues['fontFamily'] as String;

    final isMixed = mixedStyles.contains('font_weight');
    final fontWeightStyle = uniformStyles.firstWhereOrNull((m) => m['type'] == 'font_weight');
    final activeValue = isMixed
        ? null
        : (fontWeightStyle?['weight'] as int? ?? editorDefaultValues['fontWeight'] as int);

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
