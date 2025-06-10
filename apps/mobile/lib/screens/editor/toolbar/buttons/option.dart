import 'package:flutter/material.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';

class OptionToolbarButton extends StatelessWidget {
  const OptionToolbarButton({required this.component, required this.onTap, this.isActive = false, super.key});

  final Widget component;
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
          height: 48,
          child: Align(child: component),
        );
      },
    );
  }
}
