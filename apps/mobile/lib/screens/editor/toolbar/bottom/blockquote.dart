import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/widget.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/services/keyboard.dart';

class BlockquoteBottomToolbar extends HookWidget {
  const BlockquoteBottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);
    final keyboardType = useValueListenable(scope.keyboardType);

    return ListView.separated(
      padding: Pad(all: 16, bottom: MediaQuery.paddingOf(context).bottom),
      itemCount: editorValues['blockquote']!.length,
      itemBuilder: (context, index) {
        final item = editorValues['blockquote']![index];

        return WidgetToolbarButton(
          onTap: () async {
            await scope.command('blockquote', attrs: {'type': item['type']});
            switch (keyboardType) {
              case KeyboardType.software:
                await webViewController?.requestFocus();
              case KeyboardType.hardware:
                scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
            }
          },
          isActive: proseMirrorState?.isNodeActive('blockquote', attrs: {'type': item['type']}) ?? false,
          widget: SizedBox(
            height: 48,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              spacing: 8,
              children: [(item['widget'] as Widget?) ?? const SizedBox.shrink(), Text(item['label'] as String)],
            ),
          ),
        );
      },
      separatorBuilder: (context, index) {
        return const Gap(16);
      },
    );
  }
}
