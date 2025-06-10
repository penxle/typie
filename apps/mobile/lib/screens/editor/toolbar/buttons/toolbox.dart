import 'package:flutter/material.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';
import 'package:typie/widgets/svg_image.dart';

class ToolboxToolbarButton extends StatelessWidget {
  const ToolboxToolbarButton({
    required this.icon,
    required this.label,
    required this.onTap,
    this.isActive = false,
    super.key,
  });

  final String icon;
  final String label;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return ToolbarButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, color, backgroundColor) {
        return Column(
          mainAxisAlignment: MainAxisAlignment.center,
          spacing: 12,
          children: [
            SvgImage('icons/$icon', width: 28, height: 28, color: color),
            Text(label, style: TextStyle(fontSize: 15, color: color)),
          ],
        );
      },
    );
  }
}
