import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';

class LineHeightTextOptionsToolbar extends HookWidget {
  const LineHeightTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getNodeAttributes('paragraph')?['lineHeight'] as num? ?? editorDefaultValues['lineHeight'];

    return TextOptionsToolbar(
      items: editorValues['lineHeight']!,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          onTap: () async {
            await scope.command('paragraph', attrs: {'lineHeight': item['value']});
          },
        );
      },
    );
  }
}
