import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';

class FontFamilyTextOptionsToolbar extends HookWidget {
  const FontFamilyTextOptionsToolbar({super.key});

  int? _getDefaultWeight(String fontFamilyOrId, int fontWeight, GEditorScreen_QueryData? data) {
    List<int> weights = [];

    final systemFont = editorValues['fontFamily']!.firstWhereOrNull((f) => f['value'] == fontFamilyOrId);
    if (systemFont != null) {
      weights = (systemFont['weights'] as List?)?.cast<int>() ?? []
        ..sort();
    } else if (data?.me != null) {
      final userFontFamily = data!.me!.fontFamilies.firstWhereOrNull((f) => f.id == fontFamilyOrId);
      if (userFontFamily == null) {
        return null;
      }

      weights = userFontFamily.fonts.map((f) => f.weight).toList()..sort();
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
    final scope = EditorStateScope.of(context);
    final data = useValueListenable(scope.data);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
        editorDefaultValues['fontFamily'];

    final allItems = [
      ...editorValues['fontFamily']!,
      ...?data?.me?.fontFamilies.map((fontFamily) => {'value': fontFamily.id, 'label': fontFamily.name}),
    ];

    return TextOptionsToolbar(
      items: allItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () async {
            final currentWeight =
                proseMirrorState?.getMarkAttributes('text_style')?['fontWeight'] as int? ??
                (editorDefaultValues['fontWeight'] as int);

            final defaultWeight =
                _getDefaultWeight(item['value'] as String, currentWeight, data) ??
                (editorDefaultValues['fontWeight'] as int);

            await scope.command('text_style', attrs: {'fontFamily': item['value'], 'fontWeight': defaultWeight});
          },
        );
      },
    );
  }
}
