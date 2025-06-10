import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/color.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';

class TextColorTextOptionsToolbar extends HookWidget {
  const TextColorTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getMarkAttributes('text_style')?['textColor'] as String? ?? editorDefaultValues['textColor'];

    return TextOptionsToolbar(
      items: editorValues['textColor']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return ColorToolbarButton(
          hex: item['hex'] as String,
          isActive: isActive,
          onTap: () async {
            await scope.command('text_style', attrs: {'textColor': item['value']});
          },
        );
      },
    );
  }
}
