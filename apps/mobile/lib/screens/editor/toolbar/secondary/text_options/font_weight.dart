import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';

class FontWeightTextOptionsToolbar extends HookWidget {
  const FontWeightTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final data = useValueListenable(scope.data);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final currentFontFamily =
        proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
        editorDefaultValues['fontFamily'] as String;

    final activeValue =
        proseMirrorState?.getMarkAttributes('text_style')?['fontWeight'] as int? ??
        editorDefaultValues['fontWeight'] as int;

    final currentFontFamilyAndWeights = _getCurrentFontFamilyAndWeights(currentFontFamily, data);

    final availableWeightItems = editorValues['fontWeight']!
        .where((item) => currentFontFamilyAndWeights.weights.contains(item['value'] as int))
        .toList();

    return TextOptionsToolbar(
      items: availableWeightItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () async {
            await scope.command('text_style', attrs: {'fontWeight': item['value']});
          },
        );
      },
    );
  }

  ({String family, List<int> weights}) _getCurrentFontFamilyAndWeights(
    String? fontFamilyOrId,
    GEditorScreen_QueryData? data,
  ) {
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

    if (data?.me == null) {
      return defaultFontFamilyAndWeights;
    }

    final userFonts = data!.me!.fontFamilies.expand((f) => f.fonts).toList();
    if (userFonts.isEmpty) {
      return defaultFontFamilyAndWeights;
    }

    final userFontFamily = data.me!.fontFamilies.firstWhereOrNull(
      (family) => family.id == fontFamilyOrId || family.fonts.any((font) => font.id == fontFamilyOrId),
    );

    if (userFontFamily == null) {
      return defaultFontFamilyAndWeights;
    }

    return (family: userFontFamily.id, weights: userFontFamily.fonts.map((f) => f.weight).toList()..sort());
  }
}
