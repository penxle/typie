import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorLetterSpacingTextOptionsToolbar extends HookWidget {
  const NativeEditorLetterSpacingTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uniformMarks = useValueListenable(scope.uniformMarks);
    final mixedMarks = useValueListenable(scope.mixedMarks);

    final isMixed = mixedMarks.contains('letter_spacing');
    final letterSpacingMark = uniformMarks.firstWhereOrNull((m) => m['type'] == 'letter_spacing');
    final activeValue = isMixed
        ? null
        : (letterSpacingMark?['spacing'] as num? ?? editorDefaultValues['letterSpacing']);

    return NativeEditorTextOptionsToolbar(
      items: editorValues['letterSpacing']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () {
            scope.dispatch({'type': 'setLetterSpacing', 'spacing': item['value']});
          },
        );
      },
    );
  }
}
