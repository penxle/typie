import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/widget.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/services/keyboard.dart';

class HorizontalRuleBottomToolbar extends HookWidget {
  const HorizontalRuleBottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final keyboardType = useValueListenable(scope.keyboardType);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return ListView.separated(
      padding: Pad(all: 16, bottom: MediaQuery.paddingOf(context).bottom),
      itemCount: editorValues['horizontalRule']!.length,
      itemBuilder: (context, index) {
        final item = editorValues['horizontalRule']![index];

        return WidgetToolbarButton(
          onTap: () async {
            await scope.command('horizontal_rule', attrs: {'type': item['type']});
            switch (keyboardType) {
              case KeyboardType.software:
                await webViewController?.requestFocus();
              case KeyboardType.hardware:
                scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
            }
          },
          isActive: proseMirrorState?.isNodeActive('horizontal_rule', attrs: {'type': item['type']}) ?? false,
          widget: Container(height: 48, alignment: Alignment.center, child: item['widget'] as Widget),
        );
      },
      separatorBuilder: (context, index) {
        return const Gap(16);
      },
    );
  }
}
