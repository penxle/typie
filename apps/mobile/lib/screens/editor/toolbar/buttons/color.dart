import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';
import 'package:typie/styles/colors.dart';

class ColorToolbarButton extends StatelessWidget {
  const ColorToolbarButton({required this.onTap, required this.hex, this.isActive = false, super.key});

  final String hex;
  final bool isActive;
  final void Function() onTap;

  @override
  Widget build(BuildContext context) {
    final color = Color(int.parse('0xFF${hex.substring(1)}'));

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
                color: isActive ? (hex == '#ffffff' ? AppColors.gray_200 : color) : AppColors.transparent,
              ),
              borderRadius: BorderRadius.circular(999),
            ),
            child: Container(
              margin: const Pad(all: 2),
              decoration: BoxDecoration(
                color: color,
                border: Border.all(color: hex == '#ffffff' ? AppColors.gray_200 : color),
                borderRadius: BorderRadius.circular(999),
              ),
            ),
          ),
        );
      },
    );
  }
}
