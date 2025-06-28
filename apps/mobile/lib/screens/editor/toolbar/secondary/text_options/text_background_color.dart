import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/background_color.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';

class TextBackgroundColorTextOptionsToolbar extends HookWidget {
  const TextBackgroundColorTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getMarkAttributes('text_style')?['textBackgroundColor'] as String? ??
        editorDefaultValues['textBackgroundColor'];

    return TextOptionsToolbar(
      items: editorValues['textBackgroundColor']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return BackgroundColorToolbarButton(
          color: item['color'] != null ? (item['color'] as Color Function(BuildContext))(context) : null,
          value: item['value'] as String,
          isActive: isActive,
          onTap: () async {
            await scope.command('text_style', attrs: {'textBackgroundColor': item['value']});
          },
        );
      },
    );
  }
}
