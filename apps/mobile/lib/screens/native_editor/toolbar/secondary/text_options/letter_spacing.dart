import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/select_value_listenable.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';

class NativeEditorLetterSpacingTextOptionsToolbar extends HookWidget {
  const NativeEditorLetterSpacingTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final letterSpacingAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'letter_spacing'));
    final letterSpacingValues = (letterSpacingAttr?['values'] as List?)?.whereType<num>().toList() ?? [];
    final activeValue = letterSpacingValues.length == 1
        ? letterSpacingValues[0]
        : (letterSpacingValues.isEmpty ? editorDefaultValues['letterSpacing'] as num : null);

    return NativeEditorTextOptionsToolbar(
      items: editorValues['letterSpacing']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          prepareMutationOnTapDown: true,
          onTap: () {
            scope.dispatch({
              'type': 'toggleStyle',
              'style': {'type': 'letter_spacing', 'spacing': item['value']},
            });
          },
        );
      },
    );
  }
}
