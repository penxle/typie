import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';

class NativeEditorFontFamilyTextOptionsToolbar extends HookWidget {
  const NativeEditorFontFamilyTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final attrs = useValueListenable(scope.attrs);

    final fontFamilyAttr = findAttr(attrs, 'font_family');
    final fontFamilyValues = (fontFamilyAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeValue = fontFamilyValues.length == 1 ? fontFamilyValues[0] : null;

    final fontFamilies = scope.controller.fontManager?.fontFamilies ?? [];
    final activeFamilies = fontFamilies.where((f) => f.state == 'ACTIVE').toList();
    final items = activeFamilies.toList();
    if (activeValue != null && !items.any((f) => f.familyName == activeValue)) {
      final current = fontFamilies.where((f) => f.familyName == activeValue).firstOrNull;
      if (current != null) {
        items.add(current);
      }
    }
    final allItems = items.map((f) => {'label': f.displayName, 'value': f.familyName}).toList();

    return NativeEditorTextOptionsToolbar(
      items: allItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () {
            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'font_family', 'family': item['value']},
            });
          },
        );
      },
    );
  }
}
