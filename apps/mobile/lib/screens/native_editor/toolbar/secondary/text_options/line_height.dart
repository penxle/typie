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
    final attrs = useValueListenable(scope.attrs);

    final lineHeightAttr = findAttr(attrs, 'line_height');
    final lineHeightValues = (lineHeightAttr?['values'] as List?)?.whereType<num>().toList() ?? [];
    final activeValue = lineHeightValues.length == 1
        ? lineHeightValues[0]
        : (lineHeightValues.isEmpty ? editorDefaultValues['lineHeight'] : null);

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
