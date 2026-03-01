import 'package:flutter/material.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class NativeEditorCalloutFloatingToolbar extends StatelessWidget {
  const NativeEditorCalloutFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);

    return Row(
      spacing: 8,
      children: [
        FloatingToolbarButton(
          icon: LucideLightIcons.gallery_vertical_end,
          onTap: () {
            scope.dispatch({'type': 'cycleCalloutVariant'});
            scope.controller.scrollIntoView();
          },
        ),
        FloatingToolbarButton(
          icon: LucideLightIcons.text_select,
          onTap: () {
            scope.dispatch({'type': 'toggleCallout'});
            scope.controller.scrollIntoView();
          },
        ),
      ],
    );
  }
}
