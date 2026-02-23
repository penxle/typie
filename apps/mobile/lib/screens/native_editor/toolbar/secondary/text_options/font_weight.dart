import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/select_value_listenable.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorFontWeightTextOptionsToolbar extends HookWidget {
  const NativeEditorFontWeightTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final fontFamilyAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'font_family'));
    final fontFamilyValues = (fontFamilyAttr?['values'] as List?)?.whereType<String>().toList() ?? [];

    final fontWeightAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'font_weight'));
    final fontWeightValues = (fontWeightAttr?['values'] as List?)?.whereType<int>().toList() ?? [];
    final activeValue = fontWeightValues.length == 1 ? fontWeightValues[0] : null;

    final currentFonts = _getCurrentFonts(scope, fontFamilyValues, fontWeightValues);

    final weightLabelMap = <int, String>{};
    for (final item in editorValues['fontWeight']!) {
      weightLabelMap[item['value'] as int] = item['label'] as String;
    }

    final weightItems = currentFonts.map((font) {
      return {
        'value': font.weight,
        'label':
            weightLabelMap[font.weight] ??
            (font.subfamilyDisplayName != null ? '${font.subfamilyDisplayName} (${font.weight})' : '${font.weight}'),
      };
    }).toList();

    if (activeValue != null && !weightItems.any((w) => w['value'] == activeValue)) {
      weightItems
        ..add({'value': activeValue, 'label': weightLabelMap[activeValue] ?? '$activeValue'})
        ..sort((a, b) => (a['value']! as int).compareTo(b['value']! as int));
    }

    return NativeEditorTextOptionsToolbar(
      items: weightItems,
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

  List<({int weight, String? subfamilyDisplayName})> _getCurrentFonts(
    NativeEditorToolbarScope scope,
    List<String> fontFamilyValues,
    List<int> fontWeightValues,
  ) {
    final fontFamilies = scope.controller.fontManager?.fontFamilies ?? [];

    if (fontFamilyValues.length == 1) {
      final family = fontFamilies.where((f) => f.familyName == fontFamilyValues[0]).firstOrNull;
      if (family != null) {
        final activeFontsByWeight = <int, ({int weight, String? subfamilyDisplayName})>{};
        for (final f in family.fonts) {
          if (f.state == 'ACTIVE' || fontWeightValues.contains(f.weight)) {
            activeFontsByWeight[f.weight] = (weight: f.weight, subfamilyDisplayName: f.subfamilyDisplayName);
          }
        }
        final activeFonts = activeFontsByWeight.values.toList()..sort((a, b) => a.weight.compareTo(b.weight));
        return activeFonts;
      }
    }

    final fontsByWeight = <int, ({int weight, String? subfamilyDisplayName})>{};
    for (final familyName in fontFamilyValues) {
      final family = fontFamilies.where((f) => f.familyName == familyName).firstOrNull;
      if (family != null) {
        for (final font in family.fonts) {
          if (font.state == 'ACTIVE') {
            fontsByWeight[font.weight] = (weight: font.weight, subfamilyDisplayName: font.subfamilyDisplayName);
          }
        }
      }
    }

    final fonts = fontsByWeight.values.toList()..sort((a, b) => a.weight.compareTo(b.weight));
    return fonts;
  }
}
