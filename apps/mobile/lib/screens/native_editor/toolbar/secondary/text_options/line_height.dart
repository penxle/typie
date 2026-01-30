import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorLineHeightTextOptionsToolbar extends HookWidget {
  const NativeEditorLineHeightTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final selectionStats = useValueListenable(scope.selectionStats);

    final activeValue = selectionStats['uniformLineHeight'] as num? ?? editorDefaultValues['lineHeight'];

    return NativeEditorTextOptionsToolbar(
      items: editorValues['lineHeight']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () {
            scope.dispatch({'type': 'setLineHeight', 'height': item['value']});
          },
        );
      },
    );
  }
}
