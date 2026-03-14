import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/select_value_listenable.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/color.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorTextColorTextOptionsToolbar extends HookWidget {
  const NativeEditorTextColorTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final textColorAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'text_color'));
    final textColorValues = (textColorAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeValue = textColorValues.length == 1
        ? textColorValues[0]
        : (textColorValues.isEmpty ? editorDefaultValues['textColor'] : null);

    return NativeEditorTextOptionsToolbar(
      items: editorValues['textColor']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return ColorToolbarButton(
          color: (item['color'] as Color Function(BuildContext))(context),
          value: item['value'] as String,
          isActive: isActive,
          prepareMutationOnTapDown: true,
          onTap: () {
            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'text_color', 'color': item['value']},
            });
          },
        );
      },
    );
  }
}
