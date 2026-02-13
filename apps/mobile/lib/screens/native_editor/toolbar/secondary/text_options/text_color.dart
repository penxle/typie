import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/color.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorTextColorTextOptionsToolbar extends HookWidget {
  const NativeEditorTextColorTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uniformStyles = useValueListenable(scope.uniformStyles);
    final mixedStyles = useValueListenable(scope.mixedStyles);

    final isMixed = mixedStyles.contains('text_color');
    final textColorStyle = uniformStyles.firstWhereOrNull((m) => m['type'] == 'text_color');
    final activeValue = isMixed ? null : (textColorStyle?['color'] as String? ?? editorDefaultValues['textColor']);

    return NativeEditorTextOptionsToolbar(
      items: editorValues['textColor']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return ColorToolbarButton(
          color: (item['color'] as Color Function(BuildContext))(context),
          value: item['value'] as String,
          isActive: isActive,
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
