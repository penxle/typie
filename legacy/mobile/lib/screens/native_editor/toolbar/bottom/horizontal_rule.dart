import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/widget.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';
import 'package:typie/services/keyboard.dart';

class NativeEditorHorizontalRuleBottomToolbar extends HookWidget {
  const NativeEditorHorizontalRuleBottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final keyboardType = useValueListenable(scope.keyboardType);

    return ListView.separated(
      padding: Pad(all: 16, bottom: MediaQuery.paddingOf(context).bottom),
      itemCount: editorValues['horizontalRule']!.length,
      itemBuilder: (context, index) {
        final item = editorValues['horizontalRule']![index];

        return WidgetToolbarButton(
          onTap: () {
            scope.dispatch({'type': 'setHorizontalRule', 'variant': item['type']});
            scope.controller.scrollIntoView();
            switch (keyboardType) {
              case KeyboardType.software:
                scope.requestFocus();
              case KeyboardType.hardware:
                scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
            }
          },
          widget: Container(height: 48, alignment: Alignment.center, child: item['widget'] as Widget),
        );
      },
      separatorBuilder: (context, index) {
        return const Gap(16);
      },
    );
  }
}
