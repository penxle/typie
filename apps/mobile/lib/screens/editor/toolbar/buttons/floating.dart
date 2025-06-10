import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';

class FloatingToolbarButton extends StatelessWidget {
  const FloatingToolbarButton({required this.onTap, required this.icon, this.isActive = false, super.key});

  final IconData icon;

  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return ToolbarButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Container(
          decoration: BoxDecoration(
            color: backgroundColor,
            border: Border.all(color: color),
            borderRadius: BorderRadius.circular(999),
          ),
          padding: const Pad(all: 8),
          child: Icon(icon, size: 20, color: color),
        );
      },
    );
  }
}
