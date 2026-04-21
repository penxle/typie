import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/select_value_listenable.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/base.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/widgets/font_specimen.dart';

class NativeEditorFontFamilyTextOptionsToolbar extends HookWidget {
  const NativeEditorFontFamilyTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final fontFamilyAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'font_family'));
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

    final representativeFontMap = {for (final family in items) family.familyName: getRepresentativeFont(family.fonts)};

    final allItems = items.map((f) {
      final rep = representativeFontMap[f.familyName];
      return {'label': f.displayName, 'value': f.familyName, 'fontId': rep?.id};
    }).toList();

    return NativeEditorTextOptionsToolbar(
      items: allItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return ToolbarButton(
          isActive: isActive,
          color: context.colors.textFaint,
          prepareMutationOnTapDown: true,
          onTap: () {
            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'font_family', 'family': item['value']},
            });
          },
          builder: (context, color, _) {
            return Center(
              child: Container(
                padding: const Pad(all: 8),
                child: FontSpecimen(
                  text: item['label'] as String,
                  fontId: item['fontId'] as String?,
                  style: TextStyle(fontSize: 16, color: color),
                ),
              ),
            );
          },
        );
      },
    );
  }
}
