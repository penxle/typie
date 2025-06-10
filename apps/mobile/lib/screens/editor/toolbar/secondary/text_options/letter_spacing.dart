import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';

class LetterSpacingTextOptionsToolbar extends HookWidget {
  const LetterSpacingTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getNodeAttributes('paragraph')?['letterSpacing'] as num? ??
        editorDefaultValues['letterSpacing'];

    return TextOptionsToolbar(
      items: editorValues['letterSpacing']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () async {
            await scope.command('paragraph', attrs: {'letterSpacing': item['value']});
          },
        );
      },
    );
  }
}
