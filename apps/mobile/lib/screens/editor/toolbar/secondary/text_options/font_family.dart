import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';

class FontFamilyTextOptionsToolbar extends HookWidget {
  const FontFamilyTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final data = useValueListenable(scope.data);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
        editorDefaultValues['fontFamily'];

    final allItems = [
      ...editorValues['fontFamily']!,
      ...?data?.post.entity.site.fonts.map((font) => {'value': font.id, 'label': font.name}),
    ];

    return TextOptionsToolbar(
      items: allItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () async {
            await scope.command('text_style', attrs: {'fontFamily': item['value']});
          },
        );
      },
    );
  }
}
