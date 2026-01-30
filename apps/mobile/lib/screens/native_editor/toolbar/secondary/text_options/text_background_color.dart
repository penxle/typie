import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/background_color.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorTextBackgroundColorTextOptionsToolbar extends HookWidget {
  const NativeEditorTextBackgroundColorTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uniformMarks = useValueListenable(scope.uniformMarks);
    final mixedMarks = useValueListenable(scope.mixedMarks);

    final isMixed = mixedMarks.contains('background_color');
    final backgroundColorMark = uniformMarks.firstWhereOrNull((m) => m['type'] == 'background_color');
    final activeValue = isMixed
        ? null
        : (backgroundColorMark?['key'] as String? ?? editorDefaultValues['textBackgroundColor']);

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
            scope.dispatch({'type': 'toggleBackgroundColor', if (value != 'none') 'key': value});
          },
        );
      },
    );
  }
}
