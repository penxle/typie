import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';

class IconToolbarButton extends StatelessWidget {
  const IconToolbarButton({
    required this.onTap,
    required this.icon,
    this.isActive = false,
    this.isRepeatable = false,
    super.key,
  });

  final IconData icon;

  final bool isActive;
  final bool isRepeatable;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return ToolbarButton(
      isActive: isActive,
      isRepeatable: isRepeatable,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Container(
          decoration: BoxDecoration(color: backgroundColor, borderRadius: BorderRadius.circular(6)),
          padding: const Pad(all: 8),
          child: Icon(icon, size: 20, color: color),
        );
      },
    );
  }
}
