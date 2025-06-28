import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';

class ColorToolbarButton extends StatelessWidget {
  const ColorToolbarButton({
    required this.onTap,
    required this.color,
    required this.value,
    this.isActive = false,
    super.key,
  });

  final Color color;
  final String value;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    return ToolbarButton(
      isActive: isActive,
      onTap: onTap,
      builder: (context, textColor, backgroundColor) {
        return Center(
          child: Container(
            width: 26,
            height: 26,
            decoration: BoxDecoration(
              border: Border.all(
                width: 2,
                color: isActive ? (value == 'white' ? context.colors.borderDefault : color) : Colors.transparent,
              ),
              borderRadius: BorderRadius.circular(999),
            ),
            child: Container(
              margin: const Pad(all: 2),
              decoration: BoxDecoration(
                color: color,
                border: Border.all(color: value == 'white' ? context.colors.borderDefault : color),
                borderRadius: BorderRadius.circular(999),
              ),
            ),
          ),
        );
      },
    );
  }
}
