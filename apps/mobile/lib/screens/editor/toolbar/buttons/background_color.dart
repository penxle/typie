import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/editor/toolbar/buttons/base.dart';
import 'package:typie/styles/colors.dart';

class BackgroundColorToolbarButton extends StatelessWidget {
  const BackgroundColorToolbarButton({
    required this.onTap,
    required this.color,
    required this.value,
    this.isActive = false,
    super.key,
  });

  final Color? color;
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
                color: isActive
                    ? (value == 'none' ? context.colors.borderDefault : color ?? context.colors.borderDefault)
                    : AppColors.transparent,
              ),
              borderRadius: BorderRadius.circular(6),
            ),
            child: Container(
              margin: const Pad(all: 2),
              decoration: BoxDecoration(
                color: value == 'none' ? AppColors.transparent : color,
                border: Border.all(
                  color: value == 'none' ? context.colors.borderDefault : color ?? context.colors.borderDefault,
                ),
                borderRadius: BorderRadius.circular(3),
              ),
              child: value == 'none'
                  ? Center(
                      child: Transform.rotate(
                        angle: 0.785398,
                        child: Container(width: 1, height: 14, color: context.colors.textDisabled),
                      ),
                    )
                  : null,
            ),
          ),
        );
      },
    );
  }
}
