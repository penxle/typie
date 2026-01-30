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
    final uniformMarks = useValueListenable(scope.uniformMarks);
    final mixedMarks = useValueListenable(scope.mixedMarks);

    final isMixed = mixedMarks.contains('text_color');
    final textColorMark = uniformMarks.firstWhereOrNull((m) => m['type'] == 'text_color');
    final activeValue = isMixed ? null : (textColorMark?['key'] as String? ?? editorDefaultValues['textColor']);

    return NativeEditorTextOptionsToolbar(
      items: editorValues['textColor']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return ColorToolbarButton(
          color: (item['color'] as Color Function(BuildContext))(context),
          value: item['value'] as String,
          isActive: isActive,
          onTap: () {
            scope.dispatch({'type': 'toggleTextColor', 'key': item['value']});
          },
        );
      },
    );
  }
}
