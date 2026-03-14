import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/select_value_listenable.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorTextAlignTextOptionsToolbar extends HookWidget {
  const NativeEditorTextAlignTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final textAlignAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'text_align'));
    final textAlignValues = (textAlignAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeValue = textAlignValues.length == 1 ? textAlignValues[0] : editorDefaultValues['textAlign'];

    return NativeEditorTextOptionsToolbar(
      items: editorValues['textAlign']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          prepareMutationOnTapDown: true,
          onTap: () {
            scope.dispatch({'type': 'setTextAlign', 'align': item['value']});
          },
        );
      },
    );
  }
}
