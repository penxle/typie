import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';

class LabelToolbarButton extends StatelessWidget {
  const LabelToolbarButton({required this.onTap, required this.text, this.isActive = false, this.color, super.key});

  final String text;
  final Color? color;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return ToolbarButton(
      isActive: isActive,
      onTap: onTap,
      color: color,
      builder: (context, color, _) {
        return Center(
          child: Container(
            padding: const Pad(all: 8),
            child: Text(text, style: TextStyle(fontSize: 16, color: color)),
          ),
        );
      },
    );
  }
}
