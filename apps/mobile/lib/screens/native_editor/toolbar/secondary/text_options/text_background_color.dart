import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/select_value_listenable.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/background_color.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorTextBackgroundColorTextOptionsToolbar extends HookWidget {
  const NativeEditorTextBackgroundColorTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final bgColorAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'background_color'));
    final bgColorValues = (bgColorAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeValue = bgColorValues.length == 1
        ? bgColorValues[0]
        : (bgColorValues.isEmpty ? editorDefaultValues['textBackgroundColor'] : null);

    return NativeEditorTextOptionsToolbar(
      items: editorValues['textBackgroundColor']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return BackgroundColorToolbarButton(
          color: item['color'] != null ? (item['color'] as Color Function(BuildContext))(context) : null,
          value: item['value'] as String,
          isActive: isActive,
          onTap: () {
            final value = item['value'] as String;
            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'background_color', 'color': value},
            });
          },
        );
      },
    );
  }
}
