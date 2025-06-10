import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';

class CustomToolbarButton extends StatelessWidget {
  const CustomToolbarButton({required this.onTap, required this.widget, this.isActive = false, super.key});

  final Widget widget;

  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return ToolbarButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Container(
          decoration: BoxDecoration(color: backgroundColor, borderRadius: BorderRadius.circular(6)),
          padding: const Pad(all: 8),
          child: widget,
        );
      },
    );
  }
}
